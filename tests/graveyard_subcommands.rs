//! End-to-end coverage for `rclean restore`, `rclean graveyard list`,
//! and `rclean graveyard gc`.
//!
//! Each test uses `XDG_DATA_HOME=<tempdir>` so the developer's real
//! graveyard is never touched.

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

/// Spawn `rclean clean --graveyard --yes` and return the path to the
/// candidate that was buried, plus the graveyard root.
fn bury_one(workspace: &TempDir, graveyard_root: &TempDir) {
    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard_root.path())
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
}

#[test]
fn graveyard_list_table_shows_buried_candidate() {
    let workspace = TempDir::new().unwrap();
    let graveyard = TempDir::new().unwrap();
    build_node_project(&workspace);
    bury_one(&workspace, &graveyard);

    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["graveyard", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("node.node_modules"));
}

#[test]
fn graveyard_list_json_emits_manifest_record() {
    let workspace = TempDir::new().unwrap();
    let graveyard = TempDir::new().unwrap();
    build_node_project(&workspace);
    bury_one(&workspace, &graveyard);

    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["graveyard", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"rule_id\""))
        .stdout(predicate::str::contains("\"schema_version\""));
}

#[test]
fn graveyard_list_empty_returns_exit_3() {
    // Fresh graveyard, no graves yet.
    let graveyard = TempDir::new().unwrap();
    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["graveyard", "list"])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("No active graves"));
}

#[test]
fn restore_moves_payload_back_to_original_path() {
    let workspace = TempDir::new().unwrap();
    let graveyard = TempDir::new().unwrap();
    build_node_project(&workspace);
    let candidate = workspace.path().join("node_modules");
    bury_one(&workspace, &graveyard);
    assert!(!candidate.exists(), "sanity: candidate was buried");

    // Pull the id back out of the JSON list.
    let out = Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["graveyard", "list", "--json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let id = stdout
        .lines()
        .find(|line| line.trim_start().starts_with("\"id\""))
        .and_then(|line| line.split('"').nth(3))
        .expect("id field in JSON")
        .to_string();

    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["restore", "--id", &id])
        .assert()
        .success()
        .stderr(predicate::str::contains("restored"));

    assert!(
        candidate.exists(),
        "candidate should be back at its original path"
    );
    assert!(candidate.join("blob").is_file(), "payload preserved");
}

#[test]
fn restore_refuses_when_target_already_exists() {
    let workspace = TempDir::new().unwrap();
    let graveyard = TempDir::new().unwrap();
    build_node_project(&workspace);
    let candidate = workspace.path().join("node_modules");
    bury_one(&workspace, &graveyard);

    // Recreate the original path so restore can't write there.
    fs::create_dir(&candidate).unwrap();

    let out = Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["graveyard", "list", "--json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let id = stdout
        .lines()
        .find(|line| line.trim_start().starts_with("\"id\""))
        .and_then(|line| line.split('"').nth(3))
        .unwrap()
        .to_string();

    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["restore", "--id", &id])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn graveyard_gc_dry_run_reports_zero_for_fresh_graves() {
    let workspace = TempDir::new().unwrap();
    let graveyard = TempDir::new().unwrap();
    build_node_project(&workspace);
    bury_one(&workspace, &graveyard);

    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["graveyard", "gc", "--dry-run"])
        .assert()
        .success()
        .stderr(predicate::str::contains("would remove 0"));
}
