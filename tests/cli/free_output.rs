use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn build_free_fixture(temp: &TempDir) {
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules/blob"), vec![0u8; 4096]).unwrap();
}

#[test]
fn free_target_met_writes_plan_and_exits_zero() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    build_free_fixture(&temp);
    let plan_path = temp.path().join("free-plan.json");

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.args([
        "free",
        "1kb",
        temp.path().to_str().unwrap(),
        "--min-size",
        "0",
        "--write-plan",
        plan_path.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Proposed set to free"))
    .stdout(predicate::str::contains("rclean clean --plan"));

    let plan: Value = serde_json::from_str(&std::fs::read_to_string(&plan_path)?)?;
    assert_eq!(plan["deleteMode"], "trash");
    assert!(
        !plan["selected"].as_array().unwrap().is_empty(),
        "plan must carry the proposed selection"
    );
    // free never deletes anything itself.
    assert!(temp.path().join("node_modules").exists());
    Ok(())
}

#[test]
fn free_target_unmet_states_the_gap_and_exits_3() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    build_free_fixture(&temp);
    let plan_path = temp.path().join("free-plan.json");

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.args([
        "free",
        "100gb",
        temp.path().to_str().unwrap(),
        "--min-size",
        "0",
        "--write-plan",
        plan_path.to_str().unwrap(),
    ])
    .assert()
    .code(3)
    .stdout(predicate::str::contains("target not met"))
    .stdout(predicate::str::contains("short by"));

    assert!(temp.path().join("node_modules").exists());
    Ok(())
}

#[test]
fn free_with_no_candidates_exits_3() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    std::fs::write(temp.path().join("README.md"), "empty")?;

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.args([
        "free",
        "1gb",
        temp.path().to_str().unwrap(),
        "--min-size",
        "0",
    ])
    .assert()
    .code(3)
    .stdout(predicate::str::contains("no safe candidates"));
    Ok(())
}

#[test]
fn free_interactive_requires_tty_and_deletes_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    build_free_fixture(&temp);

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.args([
        "free",
        "1kb",
        temp.path().to_str().unwrap(),
        "--interactive",
        "--min-size",
        "0",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "free --interactive requires an interactive terminal",
    ));

    assert!(temp.path().join("node_modules").exists());
    Ok(())
}

#[test]
fn free_interactive_rejects_json() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    build_free_fixture(&temp);

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.args([
        "free",
        "1kb",
        temp.path().to_str().unwrap(),
        "--interactive",
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "free --interactive cannot be combined with --json",
    ));

    assert!(temp.path().join("node_modules").exists());
    Ok(())
}

#[test]
fn scan_json_stdout_stays_pure_with_progress_forced_on() {
    // RCLEAN_PROGRESS=always exercises the progress reporter even
    // without a TTY; every progress byte must land on stderr so the
    // JSON contract on stdout is unaffected.
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules/blob"), b"abc").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    let assert = cmd
        .env("RCLEAN_PROGRESS", "always")
        .args([
            "scan",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).expect("stdout must be pure JSON");
    assert!(parsed["summary"]["candidates"].as_u64().unwrap() >= 1);
    assert!(
        !stdout.contains("scanning:"),
        "progress must never reach stdout"
    );
}

#[test]
fn completions_generate_for_all_four_shells() {
    for shell in ["bash", "zsh", "fish", "powershell"] {
        let mut cmd = Command::cargo_bin("rclean").unwrap();
        let assert = cmd.args(["completions", shell]).assert().success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
        assert!(
            stdout.contains("rclean"),
            "{shell} completions must mention the binary"
        );
    }
}

#[test]
fn man_page_renders_roff() {
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["man"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".TH rclean"));
}
