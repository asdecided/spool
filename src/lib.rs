use rusqlite::{Connection, OptionalExtension, params};
use std::fs::{File, OpenOptions};
use std::path::Path;

pub mod execution;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Local job metadata. This journal does not execute or replay tools.
pub struct Journal(Connection, File);

impl Drop for Journal {
    fn drop(&mut self) {
        let _ = self.1.unlock();
    }
}

impl Journal {
    pub fn open(path: &Path) -> Result<Self> {
        // The lock file must remain in place: unlinking it would split ownership.
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path.with_extension("lock"))?;
        lock.try_lock()
            .map_err(|_| "workspace busy: another Spool process owns this journal")?;
        let mut db = Connection::open(path)?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 1 {
            return Err("journal was created by a newer Spool; upgrade before opening it".into());
        }
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch(
            "PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = FULL;
            CREATE TABLE IF NOT EXISTS jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                goal TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS checkpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id INTEGER NOT NULL REFERENCES jobs(id),
                payload TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )?;
        if version == 0 {
            let tx = db.transaction()?;
            tx.execute_batch("CREATE TABLE operations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id INTEGER NOT NULL REFERENCES jobs(id),
                argv TEXT NOT NULL,
                state TEXT NOT NULL CHECK(state IN ('prepared','dispatched','succeeded','failed','uncertain')),
                result TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                operation_id INTEGER NOT NULL REFERENCES operations(id),
                state TEXT NOT NULL,
                evidence TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            PRAGMA user_version = 1;")?;
            tx.commit()?;
        }
        Ok(Self(db, lock))
    }

    pub fn create(&self, goal: &str) -> Result<i64> {
        if goal.trim().is_empty() {
            return Err("goal must not be empty".into());
        }
        self.0
            .execute("INSERT INTO jobs(goal) VALUES (?1)", [goal])?;
        Ok(self.0.last_insert_rowid())
    }

    pub fn checkpoint(&self, job: i64, payload: &str) -> Result<i64> {
        let value: serde_json::Value = serde_json::from_str(payload)?;
        self.0.execute(
            "INSERT INTO checkpoints(job_id, payload) VALUES (?1, ?2)",
            params![job, value.to_string()],
        )?;
        Ok(self.0.last_insert_rowid())
    }

    pub fn inspect(&self, job: i64) -> Result<serde_json::Value> {
        let goal: Option<String> = self
            .0
            .query_row("SELECT goal FROM jobs WHERE id = ?1", [job], |r| r.get(0))
            .optional()?;
        let goal = goal.ok_or("job does not exist")?;
        let payload: Option<String> = self
            .0
            .query_row(
                "SELECT payload FROM checkpoints WHERE job_id = ?1 ORDER BY id DESC LIMIT 1",
                [job],
                |r| r.get(0),
            )
            .optional()?;
        let checkpoint = payload
            .map(|p| serde_json::from_str::<serde_json::Value>(&p))
            .transpose()?;
        Ok(
            serde_json::json!({"id": job, "goal": goal, "checkpoint": checkpoint, "operations": self.operations(job)?}),
        )
    }

    pub fn list(&self) -> Result<Vec<(i64, String)>> {
        let mut stmt = self.0.prepare("SELECT id, goal FROM jobs ORDER BY id")?;
        Ok(stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?)
    }
}
