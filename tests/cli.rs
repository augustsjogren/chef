use std::process::Command;

fn chef() -> Command {
    Command::new(env!("CARGO_BIN_EXE_chef"))
}

#[test]
fn help_lists_commands() {
    let output = chef().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("import"));
    assert!(stdout.contains("serve"));
    assert!(stdout.contains("stove"));
}

#[test]
fn status_uses_configured_kitchen() {
    let dir = tempfile::tempdir().unwrap();
    let kitchen = dir.path().join("kitchen");
    std::fs::create_dir_all(kitchen.join("skills")).unwrap();

    let output = chef()
        .arg("status")
        .env("CHEF_KITCHEN", &kitchen)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("skills 0"));
}
