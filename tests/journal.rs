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
