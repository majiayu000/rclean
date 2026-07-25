//! End-to-end coverage for `rclean restore`, `rclean graveyard list`,
//! and `rclean graveyard gc`.
//!
//! Each test uses `XDG_DATA_HOME=<tempdir>` so the developer's real
//! graveyard is never touched.

#![cfg(feature = "graveyard")]

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
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

fn active_records(graveyard_root: &TempDir) -> Vec<Value> {
    let output = Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard_root.path())
        .args(["graveyard", "list", "--json"])
        .output()
        .unwrap();
    assert!(matches!(output.status.code(), Some(0 | 3)));
    serde_json::from_slice(&output.stdout).unwrap()
}

fn manifest_path(graveyard_root: &TempDir) -> std::path::PathBuf {
    graveyard_root
        .path()
        .join("rclean")
        .join("graveyard")
        .join("manifest.jsonl")
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
fn restore_to_alternate_path_preserves_explicit_id_compatibility() {
    let workspace = TempDir::new().unwrap();
    let graveyard = TempDir::new().unwrap();
    let alternate_root = TempDir::new().unwrap();
    build_node_project(&workspace);
    let original = workspace.path().join("node_modules");
    bury_one(&workspace, &graveyard);
    let id = active_records(&graveyard)[0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let alternate = alternate_root.path().join("restored");

    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["restore", "--id", &id, "--to", alternate.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("restored"));

    assert!(!original.exists());
    assert!(alternate.join("blob").is_file());
    assert!(active_records(&graveyard).is_empty());
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
fn restore_without_id_requires_an_interactive_terminal_and_writes_nothing() {
    let workspace = TempDir::new().unwrap();
    let graveyard = TempDir::new().unwrap();
    build_node_project(&workspace);
    let candidate = workspace.path().join("node_modules");
    bury_one(&workspace, &graveyard);
    let manifest_before = fs::read(manifest_path(&graveyard)).unwrap();

    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .arg("restore")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "restore without --id requires an interactive terminal",
        ));

    assert!(!candidate.exists());
    assert_eq!(
        fs::read(manifest_path(&graveyard)).unwrap(),
        manifest_before
    );
}

#[test]
fn restore_id_dry_run_does_not_touch_payload_manifest_or_target_parent() {
    let workspace = TempDir::new().unwrap();
    let graveyard = TempDir::new().unwrap();
    let alternate_root = TempDir::new().unwrap();
    build_node_project(&workspace);
    let original = workspace.path().join("node_modules");
    bury_one(&workspace, &graveyard);
    let id = active_records(&graveyard)[0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let manifest_before = fs::read(manifest_path(&graveyard)).unwrap();
    let alternate = alternate_root.path().join("missing").join("restored");

    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args([
            "restore",
            "--id",
            &id,
            "--to",
            alternate.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("would attempt to restore"))
        .stdout(predicate::str::contains("payload"))
        .stdout(predicate::str::contains(alternate.to_str().unwrap()));

    assert!(!original.exists());
    assert!(!alternate.exists());
    assert!(
        !alternate.parent().unwrap().exists(),
        "dry-run must not create the target parent"
    );
    assert_eq!(
        fs::read(manifest_path(&graveyard)).unwrap(),
        manifest_before
    );
    assert_eq!(active_records(&graveyard).len(), 1);
}

#[test]
fn graveyard_list_older_than_filters_human_and_json_from_the_same_records() {
    let old_workspace = TempDir::new().unwrap();
    let fresh_workspace = TempDir::new().unwrap();
    let graveyard = TempDir::new().unwrap();
    build_node_project(&old_workspace);
    build_node_project(&fresh_workspace);
    bury_one(&old_workspace, &graveyard);
    bury_one(&fresh_workspace, &graveyard);

    let path = manifest_path(&graveyard);
    let raw = fs::read_to_string(&path).unwrap();
    let mut records = raw
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    let old_path = records[0]["original_path"].as_str().unwrap().to_string();
    let fresh_path = records[1]["original_path"].as_str().unwrap().to_string();
    records[0]["deleted_at"] = Value::String("2000-01-01T00:00:00Z".to_string());
    records[1]["deleted_at"] = Value::String("2999-01-01T00:00:00Z".to_string());
    let rewritten = records
        .iter()
        .map(|record| serde_json::to_string(record).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(&path, rewritten).unwrap();

    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["graveyard", "list", "--older-than", "30d"])
        .assert()
        .success()
        .stdout(predicate::str::contains(old_path.as_str()))
        .stdout(predicate::str::contains(fresh_path.as_str()).not());

    let output = Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["graveyard", "list", "--older-than", "30d", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let filtered: Vec<Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0]["original_path"].as_str().unwrap(), old_path);

    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["graveyard", "list", "--older-than", "100y", "--json"])
        .assert()
        .code(3)
        .stdout(predicate::eq("[]\n"));
}

#[test]
fn deferred_and_unsafe_restore_flags_are_rejected() {
    for args in [
        vec!["restore", "--since", "1h"],
        vec!["restore", "--plan", "plan-id"],
        vec!["restore", "--force"],
        vec!["restore", "--to", "/tmp/alternate"],
        vec!["graveyard", "list", "--plan", "plan-id"],
    ] {
        Command::cargo_bin("rclean")
            .unwrap()
            .args(args)
            .assert()
            .failure();
    }
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
