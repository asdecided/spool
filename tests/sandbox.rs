//! Explicit opt-in: restricted containers may not permit user/network namespaces.
use asdecided_spool::Journal;
use std::time::Duration;

#[test]
#[ignore = "requires Linux bubblewrap with working unprivileged namespaces"]
fn real_sandbox_persists_files_hides_home_and_bounds_timeout() {
    asdecided_spool::execution::doctor().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let journal = Journal::open(&dir.path().join("journal.sqlite3")).unwrap();
    let job = journal.create("sandbox acceptance").unwrap();
    journal.queue(job, &["sh".into(), "-c".into(),
        "test ! -e /root && test ! -e /work/journal.sqlite3 && test -z \"$SPOOL_SECRET\" && printf hello > proof.txt && cat proof.txt".into()]).unwrap();
    journal
        .resume(job, dir.path(), Duration::from_secs(5))
        .unwrap();
    assert_eq!(
        journal.operations(job).unwrap()[0]["result"]["stdout"],
        "hello"
    );
    journal
        .resume(job, dir.path(), Duration::from_secs(5))
        .unwrap();
    assert_eq!(
        std::fs::read(dir.path().join("workspaces/1/proof.txt")).unwrap(),
        b"hello"
    );
    let slow = journal.create("timeout").unwrap();
    journal
        .queue(slow, &["sh".into(), "-c".into(), "sleep 10".into()])
        .unwrap();
    assert!(
        journal
            .resume(slow, dir.path(), Duration::from_millis(100))
            .is_err()
    );
    assert_eq!(journal.operations(slow).unwrap()[0]["state"], "uncertain");
}
