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

#[test]
fn status_hints_when_kitchen_missing() {
    let dir = tempfile::tempdir().unwrap();
    let output = chef()
        .arg("status")
        .env("CHEF_KITCHEN", dir.path().join("nope"))
        .output()
        .unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no kitchen"), "stderr: {stderr}");
    assert!(stderr.contains("CHEF_KITCHEN"), "stderr: {stderr}");
}

#[cfg(unix)]
#[test]
fn bootstrap_links_bundled_skills_without_kitchen_copies() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let kitchen = dir.path().join("kitchen");
    std::fs::create_dir_all(kitchen.join("skills")).unwrap();
    std::fs::create_dir_all(&home).unwrap();

    let output = chef()
        .args(["bootstrap", "--harness", "claude-user"])
        .env("HOME", &home)
        .env("CHEF_KITCHEN", &kitchen)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in ["chef", "chef-add-skill"] {
        let link = home.join(".claude/skills").join(name);
        let target = std::fs::read_link(&link).unwrap();
        assert!(
            target.ends_with(format!("bootstrap/skills/{name}")),
            "{target:?}"
        );
    }
}
