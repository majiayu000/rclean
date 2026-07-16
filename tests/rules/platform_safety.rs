use super::common::{make_dir, make_non_empty_path};
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn xcode_derived_data_is_classified_under_library_developer_xcode() {
    // Simulate the canonical Xcode path layout. The scan root is the
    // synthetic `Library/Developer/Xcode` directory; the candidate is
    // `DerivedData` directly inside it. We don't rely on the user
    // actually having Xcode installed.
    let temp = TempDir::new().unwrap();
    let xcode_dir = temp.path().join("Library").join("Developer").join("Xcode");
    fs::create_dir_all(&xcode_dir).unwrap();
    make_dir(&xcode_dir, "DerivedData");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        xcode_dir.to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "\"ruleId\": \"xcode.derived_data\"",
    ))
    .stdout(predicate::str::contains("\"safety\": \"safe\""))
    .stdout(predicate::str::contains("\"category\": \"build\""));
}

#[test]
fn xcode_derived_data_safe_candidate_can_be_cleaned() {
    let temp = TempDir::new().unwrap();
    let xcode_dir = temp.path().join("Library").join("Developer").join("Xcode");
    let derived_data = xcode_dir.join("DerivedData");
    fs::create_dir_all(&derived_data).unwrap();
    fs::write(derived_data.join("placeholder"), b"x").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "clean",
        xcode_dir.to_str().unwrap(),
        "--all",
        "--permanent",
        "--yes",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Cleaned: 1 candidates"));

    assert!(!derived_data.exists(), "DerivedData should be removed");
}

#[test]
fn xcode_derived_data_action_plan_replays_successfully() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let xcode_dir = temp.path().join("Library").join("Developer").join("Xcode");
    let derived_data = xcode_dir.join("DerivedData");
    fs::create_dir_all(&derived_data)?;
    fs::write(derived_data.join("placeholder"), b"x")?;
    let plan = temp.path().join("plan.json");

    let mut scan = Command::cargo_bin("rclean").unwrap();
    scan.arg("scan")
        .arg(&xcode_dir)
        .arg("--write-plan")
        .arg(&plan)
        .args(["--min-size", "0"])
        .assert()
        .success();

    let mut clean = Command::cargo_bin("rclean").unwrap();
    clean
        .arg("clean")
        .arg("--plan")
        .arg(&plan)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan: 1 candidates"));

    assert!(derived_data.exists(), "dry-run must not remove DerivedData");
    Ok(())
}

#[test]
fn xcode_simulators_is_classified_under_library_developer() {
    let temp = TempDir::new().unwrap();
    let developer = temp.path().join("Library").join("Developer");
    fs::create_dir_all(&developer).unwrap();
    make_dir(&developer, "CoreSimulator");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        developer.to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"ruleId\": \"xcode.simulators\""))
    .stdout(predicate::str::contains("\"safety\": \"caution\""))
    .stdout(predicate::str::contains("\"category\": \"cache\""));
}

#[test]
fn xcode_derived_data_outside_canonical_path_is_not_classified() {
    // A directory literally named `DerivedData` outside the canonical
    // Xcode path must not be picked up.
    let temp = TempDir::new().unwrap();
    make_dir(temp.path(), "DerivedData");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        temp.path().to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    // exit code 3 = no candidates (matches main.rs Commands::Scan).
    .code(3)
    .stdout(predicate::str::contains("\"ruleId\": \"xcode.derived_data\"").not());
}

#[test]
fn docker_daemon_storage_candidates_are_blocked() {
    let temp = TempDir::new().unwrap();
    let project = temp
        .path()
        .join("var")
        .join("lib")
        .join("docker")
        .join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
    make_non_empty_path(&project.join("target"));

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        temp.path().to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
        "--include-blocked",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"ruleId\": \"rust.target\""))
    .stdout(predicate::str::contains("\"safety\": \"blocked\""))
    .stdout(predicate::str::contains(
        "candidate is inside Docker daemon storage",
    ));
}
