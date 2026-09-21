use asdecided_spool::Journal;

#[test]
fn checkpoint_survives_reopening_and_jobs_remain_isolated() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.sqlite3");
    let first;
    let second;
    {
        let journal = Journal::open(&path).unwrap();
        first = journal.create("Investigate CI").unwrap();
        second = journal.create("Prepare release").unwrap();
        journal.checkpoint(first, r#"{"step":1}"#).unwrap();
        journal.checkpoint(first, r#"{"step":2}"#).unwrap();
    }
    let journal = Journal::open(&path).unwrap();
    assert_eq!(journal.inspect(first).unwrap()["checkpoint"]["step"], 2);
    assert!(journal.inspect(second).unwrap()["checkpoint"].is_null());
    assert_eq!(journal.list().unwrap().len(), 2);
}

#[test]
fn invalid_updates_do_not_replace_the_last_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let journal = Journal::open(&dir.path().join("journal.sqlite3")).unwrap();
    let job = journal.create("Task").unwrap();
    journal.checkpoint(job, r#"{"ok":true}"#).unwrap();
    assert!(journal.checkpoint(job, "invalid json").is_err());
    assert!(journal.checkpoint(job + 1, "{}").is_err());
    assert!(journal.create("  ").is_err());
    assert!(journal.inspect(job + 1).is_err());
    assert_eq!(journal.inspect(job).unwrap()["checkpoint"]["ok"], true);
}

#[test]
fn legacy_database_migrates_without_losing_checkpoints() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.sqlite3");
    {
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE jobs(id INTEGER PRIMARY KEY AUTOINCREMENT,goal TEXT NOT NULL,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
            CREATE TABLE checkpoints(id INTEGER PRIMARY KEY AUTOINCREMENT,job_id INTEGER NOT NULL REFERENCES jobs(id),payload TEXT NOT NULL,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
            INSERT INTO jobs(goal) VALUES ('legacy');
            INSERT INTO checkpoints(job_id,payload) VALUES (1,'{\"saved\":true}');").unwrap();
    }
    let journal = Journal::open(&path).unwrap();
    assert_eq!(journal.inspect(1).unwrap()["checkpoint"]["saved"], true);
    journal.queue(1, &["true".into()]).unwrap();
    drop(journal);
    let journal = Journal::open(&path).unwrap();
    assert_eq!(journal.operations(1).unwrap().len(), 1);
}

#[test]
fn newer_schema_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.sqlite3");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("PRAGMA user_version=99;").unwrap();
    assert!(Journal::open(&path).is_err());
    let version: i64 = db
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 99);
}
