use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn help_prints_usage() {
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Find and clean rebuildable"));
}

#[test]
fn scan_json_detects_node_modules() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        temp.path().to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "\"ruleId\": \"node.node_modules\"",
    ));
}

#[test]
fn clean_dry_run_does_not_delete() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "clean",
        temp.path().to_str().unwrap(),
        "--all",
        "--dry-run",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Plan:"));

    assert!(temp.path().join("node_modules").exists());
}

#[test]
fn clean_permanent_yes_deletes_safe_candidate() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "clean",
        temp.path().to_str().unwrap(),
        "--all",
        "--permanent",
        "--yes",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Cleaned: 1"));

    assert!(!temp.path().join("node_modules").exists());
}

#[test]
fn scan_write_plan_then_clean_plan_dry_run() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();
    let plan = temp.path().join("plan.json");

    let mut scan = Command::cargo_bin("rclean").unwrap();
    scan.args([
        "scan",
        temp.path().to_str().unwrap(),
        "--write-plan",
        plan.to_str().unwrap(),
        "--min-size",
        "0",
    ])
    .assert()
    .success();

    assert!(plan.exists());

    let mut clean = Command::cargo_bin("rclean").unwrap();
    clean
        .args(["clean", "--plan", plan.to_str().unwrap(), "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan: 1 candidates"));

    assert!(temp.path().join("node_modules").exists());
}

#[test]
fn clean_interactive_selection_accepts_number() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "clean",
        temp.path().to_str().unwrap(),
        "--dry-run",
        "--min-size",
        "0",
    ])
    .write_stdin("1\n")
    .assert()
    .success()
    .stdout(predicate::str::contains("Select candidates"))
    .stdout(predicate::str::contains("Plan: 1 candidates"));

    assert!(temp.path().join("node_modules").exists());
}
