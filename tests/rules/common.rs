use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

pub(super) fn make_dir(parent: &Path, name: &str) {
    fs::create_dir(parent.join(name)).unwrap();
    // Ensure non-empty so dir_size > 0 and the candidate isn't size-filtered out.
    fs::write(parent.join(name).join("placeholder"), b"x").unwrap();
}

pub(super) fn make_non_empty_path(path: &Path) {
    fs::create_dir_all(path).unwrap();
    fs::write(path.join("placeholder"), b"x").unwrap();
}

pub(super) fn scan_and_expect_rule(temp: &TempDir, expected_rule: &str) {
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
    .stdout(predicate::str::contains(format!(
        "\"ruleId\": \"{expected_rule}\""
    )));
}
