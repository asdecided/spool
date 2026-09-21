use std::process::Command;

#[test]
fn separate_processes_share_checkpoints() {
    let dir = tempfile::tempdir().unwrap();
    let invoke = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_spool"))
            .current_dir(dir.path())
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    invoke(&["init"]);
    let id = invoke(&["new", "Keep working"]);
    invoke(&["checkpoint", id.trim(), r#"{"next":"test"}"#]);
    let result: serde_json::Value = serde_json::from_str(&invoke(&["inspect", id.trim()])).unwrap();
    assert_eq!(result["checkpoint"]["next"], "test");
}
