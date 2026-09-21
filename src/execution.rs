use crate::{Journal, Result};
use rusqlite::params;
use serde_json::{Value, json};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const OUTPUT_LIMIT: usize = 64 * 1024;

impl Journal {
    pub fn queue(&self, job: i64, argv: &[String]) -> Result<i64> {
        if argv.is_empty() || argv[0].is_empty() {
            return Err("command must not be empty".into());
        }
        let tx = self.0.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO operations(job_id, argv, state) VALUES (?1, ?2, 'prepared')",
            params![job, serde_json::to_string(argv)?],
        )?;
        let id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO events(operation_id, state, evidence) VALUES (?1, 'prepared', '{}')",
            [id],
        )?;
        tx.commit()?;
        Ok(id)
    }

    pub fn operations(&self, job: i64) -> Result<Vec<Value>> {
        let mut stmt = self.0.prepare(
            "SELECT id, argv, state, result FROM operations WHERE job_id=?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([job], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })?;
        rows.map(|row| {
            let (id, argv, state, result) = row?;
            Ok(
                json!({"id":id,"argv":serde_json::from_str::<Value>(&argv)?,"state":state,
                "result":result.map(|s|serde_json::from_str::<Value>(&s)).transpose()?}),
            )
        })
        .collect()
    }

    /// Atomically records a state transition and its evidence.
    fn transition(&self, id: i64, from: &str, to: &str, evidence: Value) -> Result<()> {
        let tx = self.0.unchecked_transaction()?;
        let changed = tx.execute(
            "UPDATE operations SET state=?1, result=?2 WHERE id=?3 AND state=?4",
            params![to, evidence.to_string(), id, from],
        )?;
        if changed != 1 {
            return Err("operation is missing or no longer in the required state".into());
        }
        tx.execute(
            "INSERT INTO events(operation_id, state, evidence) VALUES (?1, ?2, ?3)",
            params![id, to, evidence.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Only the journal owner can recover. Never replays work.
    pub fn recover(&self) -> Result<usize> {
        let tx = self.0.unchecked_transaction()?;
        tx.execute("INSERT INTO events(operation_id,state,evidence) SELECT id,'uncertain','{\"reason\":\"owner interrupted before recording outcome\"}' FROM operations WHERE state='dispatched'", [])?;
        let count = tx.execute("UPDATE operations SET state='uncertain',result='{\"reason\":\"owner interrupted before recording outcome\"}' WHERE state='dispatched'", [])?;
        tx.commit()?;
        Ok(count)
    }

    pub fn resolve(&self, id: i64, outcome: &str, note: &str) -> Result<()> {
        if !["succeeded", "failed"].contains(&outcome) || note.trim().is_empty() {
            return Err("resolve requires succeeded|failed and a nonempty evidence note".into());
        }
        self.transition(id, "uncertain", outcome, json!({"manual_resolution":note}))
    }

    pub fn events(&self, id: i64) -> Result<Vec<Value>> {
        let mut stmt = self.0.prepare(
            "SELECT state,evidence,created_at FROM events WHERE operation_id=?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        rows.map(|row| {
            let (state, evidence, at) = row?;
            Ok(json!({"state":state,"evidence":serde_json::from_str::<Value>(&evidence)?,"at":at}))
        })
        .collect()
    }

    /// Execute prepared operations in order, stopping at failures or uncertainty.
    pub fn resume(&self, job: i64, root: &Path, timeout: Duration) -> Result<()> {
        self.inspect(job)?;
        self.recover()?;
        let operations = self.operations(job)?;
        if operations
            .iter()
            .any(|op| ["uncertain", "failed"].contains(&op["state"].as_str().unwrap_or("")))
        {
            return Err("job contains failed or uncertain operations; inspect it before continuing (create a new job after a failure)".into());
        }
        for op in operations {
            if op["state"] != "prepared" {
                continue;
            }
            let argv: Vec<String> = serde_json::from_value(op["argv"].clone())?;
            let id = op["id"].as_i64().ok_or("invalid operation id")?;
            let workspace = root.join("workspaces").join(job.to_string());
            std::fs::create_dir_all(&workspace)?;
            let mut command = sandbox_command(&workspace, &argv)?;
            self.transition(
                id,
                "prepared",
                "dispatched",
                json!({"timeout_seconds":timeout.as_secs()}),
            )?;
            let outcome = capture(&mut command, timeout);
            match outcome {
                Ok(result) => {
                    // A nonzero exit may have made partial changes. It is never retried automatically.
                    let state = if result["timed_out"] == true {
                        "uncertain"
                    } else if result["exit_code"] == 0 {
                        "succeeded"
                    } else {
                        "failed"
                    };
                    self.transition(id, "dispatched", state, result)?;
                    if state != "succeeded" {
                        return Err(
                            format!("operation {id} {state}; inspect the job for output").into(),
                        );
                    }
                }
                Err(error) => {
                    self.transition(
                        id,
                        "dispatched",
                        "uncertain",
                        json!({"error":error.to_string()}),
                    )?;
                    return Err(error);
                }
            }
        }
        Ok(())
    }
}

fn sandbox_command(workspace: &Path, argv: &[String]) -> Result<Command> {
    if !cfg!(target_os = "linux") {
        return Err("execution requires Linux and bubblewrap".into());
    }
    let mut command = Command::new("/usr/bin/bwrap");
    command.env_clear().args([
        "--unshare-all",
        "--die-with-parent",
        "--new-session",
        "--cap-drop",
        "ALL",
        "--ro-bind",
        "/usr",
        "/usr",
        "--symlink",
        "usr/bin",
        "/bin",
        "--symlink",
        "usr/bin",
        "/sbin",
    ]);
    for path in ["/lib", "/lib64"] {
        if Path::new(path).exists() {
            command.args(["--ro-bind", path, path]);
        }
    }
    command
        .args([
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--dir",
            "/home",
            "--setenv",
            "HOME",
            "/home",
            "--setenv",
            "PATH",
            "/usr/bin:/bin",
            "--setenv",
            "LANG",
            "C.UTF-8",
            "--bind",
        ])
        .arg(workspace.canonicalize()?)
        .args(["/work", "--chdir", "/work", "--"])
        .args(argv);
    Ok(command)
}

fn read_bounded(mut reader: impl Read) -> std::io::Result<(Vec<u8>, bool)> {
    let mut output = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut truncated = false;
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        let keep = n.min(OUTPUT_LIMIT - output.len());
        output.extend_from_slice(&buffer[..keep]);
        truncated |= keep < n;
    }
    Ok((output, truncated))
}

fn capture(command: &mut Command, timeout: Duration) -> Result<Value> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().ok_or("missing stdout")?;
    let stderr = child.stderr.take().ok_or("missing stderr")?;
    let out = std::thread::spawn(move || read_bounded(stdout));
    let err = std::thread::spawn(move || read_bounded(stderr));
    let start = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() >= timeout {
            timed_out = true;
            child.kill()?;
            break child.wait()?;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let (stdout, out_truncated) = out.join().map_err(|_| "stdout reader panicked")??;
    let (stderr, err_truncated) = err.join().map_err(|_| "stderr reader panicked")??;
    Ok(
        json!({"exit_code":status.code(),"timed_out":timed_out,"stdout":String::from_utf8_lossy(&stdout),
        "stderr":String::from_utf8_lossy(&stderr),"stdout_truncated":out_truncated,"stderr_truncated":err_truncated}),
    )
}

pub fn doctor() -> Result<Value> {
    let workspace = std::env::current_dir()?;
    let mut cmd = sandbox_command(&workspace, &["/bin/true".to_string()])?;
    let result = capture(&mut cmd, Duration::from_secs(5))?;
    if result["exit_code"] != 0 {
        return Err(format!("sandbox unavailable: {}", result["stderr"]).into());
    }
    Ok(json!({"sandbox":"bubblewrap","available":true,"network":false}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_is_bounded_and_drained() {
        let data = vec![b'x'; OUTPUT_LIMIT + 500];
        let (bytes, truncated) = read_bounded(&data[..]).unwrap();
        assert_eq!(bytes.len(), OUTPUT_LIMIT);
        assert!(truncated);
    }

    // This helper exists only in the test executable, never the shipped CLI.
    #[test]
    #[ignore]
    fn crash_worker() {
        let root = std::env::var("SPOOL_TEST_ROOT").unwrap();
        let phase = std::env::var("SPOOL_TEST_PHASE").unwrap();
        let path = Path::new(&root);
        let journal = Journal::open(&path.join("journal.sqlite3")).unwrap();
        let job = journal.create("Crash test").unwrap();
        let op = journal.queue(job, &["fake-effect".into()]).unwrap();
        if phase != "prepared" {
            journal
                .transition(op, "prepared", "dispatched", json!({}))
                .unwrap();
        }
        if phase == "effect" || phase == "completed" {
            let mut file = std::fs::File::create(path.join("external-effect")).unwrap();
            std::io::Write::write_all(&mut file, b"once").unwrap();
            file.sync_all().unwrap();
        }
        if phase == "completed" {
            journal
                .transition(op, "dispatched", "succeeded", json!({"receipt":"once"}))
                .unwrap();
        }
        std::fs::write(path.join("ready"), b"ready").unwrap();
        loop {
            std::thread::park();
        }
    }

    #[test]
    fn killed_workers_preserve_outcomes_and_never_replay_uncertainty() {
        for phase in ["prepared", "dispatched", "effect", "completed"] {
            let dir = tempfile::tempdir().unwrap();
            let mut child = Command::new(std::env::current_exe().unwrap())
                .args(["--ignored", "--exact", "execution::tests::crash_worker"])
                .env("SPOOL_TEST_ROOT", dir.path())
                .env("SPOOL_TEST_PHASE", phase)
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();
            let start = Instant::now();
            while !dir.path().join("ready").exists() {
                if start.elapsed() > Duration::from_secs(10) {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("worker did not become ready");
                }
                assert!(child.try_wait().unwrap().is_none(), "worker exited early");
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(Journal::open(&dir.path().join("journal.sqlite3")).is_err());
            child.kill().unwrap();
            child.wait().unwrap();
            let journal = Journal::open(&dir.path().join("journal.sqlite3")).unwrap();
            let count = journal.recover().unwrap();
            let expected = match phase {
                "prepared" => "prepared",
                "completed" => "succeeded",
                _ => "uncertain",
            };
            assert_eq!(journal.operations(1).unwrap()[0]["state"], expected);
            assert_eq!(count, usize::from(expected == "uncertain"));
            assert_eq!(journal.recover().unwrap(), 0);
            if expected == "uncertain" {
                assert!(
                    journal
                        .resume(1, dir.path(), Duration::from_secs(1))
                        .is_err()
                );
                assert!(
                    journal
                        .transition(1, "prepared", "dispatched", json!({}))
                        .is_err()
                );
                journal
                    .resolve(
                        1,
                        "succeeded",
                        "Inspected fake destination; effect reconciled",
                    )
                    .unwrap();
                assert!(journal.resolve(1, "succeeded", "again").is_err());
                assert_eq!(
                    journal.events(1).unwrap().last().unwrap()["state"],
                    "succeeded"
                );
            }
            if phase == "effect" || phase == "completed" {
                assert_eq!(
                    std::fs::read(dir.path().join("external-effect")).unwrap(),
                    b"once"
                );
            } else {
                assert!(!dir.path().join("external-effect").exists());
            }
        }
    }
}
