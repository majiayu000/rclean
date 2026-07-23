//! End-to-end coverage for `clean --graveyard` through the binary.
//!
//! Gated behind `--features graveyard`; `cargo test --no-default-
//! features` correctly skips this file. Uses a custom
//! `XDG_DATA_HOME` so the test never touches the developer's real
//! graveyard.

#![cfg(feature = "graveyard")]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn build_node_project(temp: &TempDir) {
    fs::write(temp.path().join("package.json"), "{}").unwrap();
    fs::create_dir(temp.path().join("node_modules")).unwrap();
    fs::write(temp.path().join("node_modules").join("blob"), b"abc").unwrap();
}

#[test]
fn clean_graveyard_moves_candidate_and_writes_manifest() {
    let workspace = TempDir::new().unwrap();
    let graveyard_root = TempDir::new().unwrap();
    build_node_project(&workspace);

    let candidate = workspace.path().join("node_modules");
    assert!(candidate.exists(), "fixture sanity: candidate should exist");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.env("XDG_DATA_HOME", graveyard_root.path())
        .args([
            "clean",
            workspace.path().to_str().unwrap(),
            "--all",
            "--graveyard",
            "--yes",
            "--min-size",
            "0",
        ])
        .assert()
        .success();

    // Original candidate gone from the workspace.
    assert!(
        !candidate.exists(),
        "candidate should have been moved into the graveyard"
    );

    // Manifest exists under $XDG_DATA_HOME/rclean/graveyard/.
    let manifest = graveyard_root
        .path()
        .join("rclean")
        .join("graveyard")
        .join("manifest.jsonl");
    assert!(manifest.is_file(), "manifest.jsonl should be written");

    let body = fs::read_to_string(&manifest).unwrap();
    assert!(body.contains("\"node.node_modules\""));
    assert!(body.contains("\"safety_at_delete\":\"safe\""));
}

#[test]
fn graveyard_and_permanent_flags_are_mutually_exclusive() {
    // clap should reject this combination at arg-parse time without
    // ever running the scan / clean code path.
    let workspace = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "clean",
        workspace.path().to_str().unwrap(),
        "--all",
        "--graveyard",
        "--permanent",
        "--yes",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("--graveyard").or(predicate::str::contains("--permanent")));
}

#[test]
fn clean_graveyard_write_plan_marks_delete_mode() {
    let workspace = TempDir::new().unwrap();
    build_node_project(&workspace);
    let plan = workspace.path().join("plan.json");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "clean",
        workspace.path().to_str().unwrap(),
        "--all",
        "--dry-run",
        "--graveyard",
        "--write-plan",
        plan.to_str().unwrap(),
        "--min-size",
        "0",
    ])
    .assert()
    .success();

    let raw = fs::read_to_string(plan).unwrap();
    assert!(raw.contains(r#""schemaVersion": 2"#));
    assert!(raw.contains(r#""deleteMode": "graveyard""#));
    assert!(raw.contains(r#""id": "#));
    assert!(raw.contains(r#""riskScore": "#));
}

#[test]
fn clean_graveyard_plan_replays_into_graveyard_without_flag() {
    let workspace = TempDir::new().unwrap();
    let graveyard_root = TempDir::new().unwrap();
    build_node_project(&workspace);
    let plan = workspace.path().join("plan.json");
    let candidate = workspace.path().join("node_modules");

    let mut write_plan = Command::cargo_bin("rclean").unwrap();
    write_plan
        .env("XDG_DATA_HOME", graveyard_root.path())
        .args([
            "clean",
            workspace.path().to_str().unwrap(),
            "--all",
            "--dry-run",
            "--graveyard",
            "--write-plan",
            plan.to_str().unwrap(),
            "--min-size",
            "0",
        ])
        .assert()
        .success();

    let mut replay = Command::cargo_bin("rclean").unwrap();
    replay
        .env("XDG_DATA_HOME", graveyard_root.path())
        .args(["clean", "--plan", plan.to_str().unwrap(), "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleaned: 1"));

    assert!(
        !candidate.exists(),
        "candidate should have been moved into the graveyard"
    );
    let manifest = graveyard_root
        .path()
        .join("rclean")
        .join("graveyard")
        .join("manifest.jsonl");
    assert!(
        manifest.is_file(),
        "plan replay should honor deleteMode=graveyard"
    );
    let body = fs::read_to_string(&manifest).unwrap();
    assert!(body.contains("\"node.node_modules\""));
}

#[test]
fn clean_plan_rejects_conflicting_cli_delete_mode() {
    let workspace = TempDir::new().unwrap();
    let graveyard_root = TempDir::new().unwrap();
    build_node_project(&workspace);
    let plan = workspace.path().join("plan.json");
    let candidate = workspace.path().join("node_modules");

    let mut write_plan = Command::cargo_bin("rclean").unwrap();
    write_plan
        .env("XDG_DATA_HOME", graveyard_root.path())
        .args([
            "clean",
            workspace.path().to_str().unwrap(),
            "--all",
            "--dry-run",
            "--graveyard",
            "--write-plan",
            plan.to_str().unwrap(),
            "--min-size",
            "0",
        ])
        .assert()
        .success();

    let mut replay = Command::cargo_bin("rclean").unwrap();
    replay
        .env("XDG_DATA_HOME", graveyard_root.path())
        .args([
            "clean",
            "--plan",
            plan.to_str().unwrap(),
            "--permanent",
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("conflicts with action plan deleteMode")
                .and(predicate::str::contains("graveyard"))
                .and(predicate::str::contains("permanent")),
        );

    assert!(
        candidate.exists(),
        "conflicting mode must fail before deleting the candidate"
    );
}

#[test]
fn clean_graveyard_prints_recovery_summary() {
    let workspace = TempDir::new().unwrap();
    let graveyard_root = TempDir::new().unwrap();
    build_node_project(&workspace);

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.env("XDG_DATA_HOME", graveyard_root.path())
        .args([
            "clean",
            workspace.path().to_str().unwrap(),
            "--all",
            "--graveyard",
            "--yes",
            "--min-size",
            "0",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("recoverable for 7 days"))
        .stdout(predicate::str::contains("rclean restore --id <id>"));
}
