//! Mutating commands must corroborate manifest records with per-grave metadata.
#![cfg(feature = "graveyard")]

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn bury_pair(temp: &TempDir) -> (PathBuf, Vec<Value>) {
    let data = temp.path().join("data");
    for name in ["first", "second"] {
        let project = temp.path().join(name);
        fs::create_dir_all(project.join("node_modules")).unwrap();
        fs::write(project.join("package.json"), "{}").unwrap();
        fs::write(project.join("node_modules/blob"), name).unwrap();
        Command::cargo_bin("rclean")
            .unwrap()
            .env("XDG_DATA_HOME", &data)
            .args([
                "clean",
                project.to_str().unwrap(),
                "--all",
                "--graveyard",
                "--yes",
                "--min-size",
                "0",
            ])
            .assert()
            .success();
    }
    let root = data.join("rclean/graveyard");
    let records = fs::read_to_string(root.join("manifest.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    (root, records)
}

fn write_manifest(root: &Path, records: &[Value]) {
    let body = records
        .iter()
        .map(|record| serde_json::to_string(record).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(root.join("manifest.jsonl"), body).unwrap();
}

fn assert_refused(
    temp: &TempDir,
    root: &Path,
    originals: &[Value],
    records: &[Value],
    restore: bool,
    error: &str,
) {
    write_manifest(root, records);
    let manifest_before = fs::read(root.join("manifest.jsonl")).unwrap();
    let target = temp.path().canonicalize().unwrap().join("missing/restored");
    let mut command = Command::cargo_bin("rclean").unwrap();
    command.env("XDG_DATA_HOME", temp.path().join("data"));
    if restore {
        command.args([
            "restore",
            "--id",
            records[0]["id"].as_str().unwrap(),
            "--to",
            target.to_str().unwrap(),
        ]);
    } else {
        command
            .args(["graveyard", "gc"])
            .assert()
            .failure()
            .stderr(predicate::str::contains(error));
        command = Command::cargo_bin("rclean").unwrap();
        command.env("XDG_DATA_HOME", temp.path().join("data"));
        command.args(["graveyard", "gc", "--dry-run"]);
    }
    command
        .assert()
        .failure()
        .stderr(predicate::str::contains(error));
    assert!(
        !target.parent().unwrap().exists(),
        "refusal must precede creating restore parents"
    );
    assert_eq!(
        fs::read(root.join("manifest.jsonl")).unwrap(),
        manifest_before
    );
    for (record, contents) in originals.iter().zip(["first", "second"]) {
        assert_eq!(
            fs::read(
                root.join(record["grave_path"].as_str().unwrap())
                    .join("payload/blob")
            )
            .unwrap(),
            contents.as_bytes()
        );
        assert!(!Path::new(record["original_path"].as_str().unwrap()).exists());
    }
}

fn assert_forgery_refused(restore: bool, copy_identity: bool) {
    let temp = TempDir::new().unwrap();
    let (root, originals) = bury_pair(&temp);
    let mut records = originals.clone();
    if copy_identity {
        for field in ["id", "deleted_at", "grave_path"] {
            records[0][field] = records[1][field].clone();
        }
    }
    records[0]["expires_at"] = json!("2000-01-01T00:00:00Z");
    assert_refused(
        &temp,
        &root,
        &originals,
        &records,
        restore,
        "does not safely name",
    );
}

#[test]
fn gc_refuses_forged_identity_before_deleting_another_grave() {
    assert_forgery_refused(false, true);
}

#[test]
fn restore_refuses_forged_identity_before_moving_another_payload() {
    assert_forgery_refused(true, true);
}

#[test]
fn gc_refuses_manifest_only_expiry_change() {
    assert_forgery_refused(false, false);
}

#[test]
fn restore_refuses_manifest_only_expiry_change() {
    assert_forgery_refused(true, false);
}

#[test]
fn gc_checks_later_metadata_before_deleting_a_valid_expired_grave() {
    let temp = TempDir::new().unwrap();
    let (root, originals) = bury_pair(&temp);
    let mut records = originals.clone();
    records[0]["expires_at"] = json!("2000-01-01T00:00:00Z");
    fs::write(
        root.join(records[0]["grave_path"].as_str().unwrap())
            .join("meta.json"),
        serde_json::to_vec(&records[0]).unwrap(),
    )
    .unwrap();
    for field in ["id", "deleted_at", "grave_path", "expires_at"] {
        records[1][field] = records[0][field].clone();
    }
    assert_refused(
        &temp,
        &root,
        &originals,
        &records,
        false,
        "does not safely name",
    );
}

#[test]
fn missing_metadata_refuses_mutations_of_existing_graves() {
    let temp = TempDir::new().unwrap();
    let (root, originals) = bury_pair(&temp);
    let mut records = originals.clone();
    records[0]["expires_at"] = json!("2000-01-01T00:00:00Z");
    fs::remove_file(
        root.join(records[0]["grave_path"].as_str().unwrap())
            .join("meta.json"),
    )
    .unwrap();
    for restore in [true, false] {
        assert_refused(
            &temp,
            &root,
            &originals,
            &records,
            restore,
            "graveyard io at",
        );
    }
}

#[test]
fn malformed_metadata_refuses_mutations_of_existing_graves() {
    let temp = TempDir::new().unwrap();
    let (root, originals) = bury_pair(&temp);
    let mut records = originals.clone();
    records[0]["expires_at"] = json!("2000-01-01T00:00:00Z");
    fs::write(
        root.join(records[0]["grave_path"].as_str().unwrap())
            .join("meta.json"),
        "{",
    )
    .unwrap();
    for restore in [true, false] {
        assert_refused(
            &temp,
            &root,
            &originals,
            &records,
            restore,
            "graveyard manifest parse:",
        );
    }
}
