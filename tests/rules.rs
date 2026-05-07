//! Per-ecosystem rule tests via `rclean scan --json`.
//!
//! These exercise `classify_candidate` end-to-end through the binary because
//! the crate has no `lib.rs`. Each test sets up a minimal project marker plus
//! the candidate dir, then asserts the JSON output names the expected rule_id.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn make_dir(parent: &Path, name: &str) {
    fs::create_dir(parent.join(name)).unwrap();
    // Ensure non-empty so dir_size > 0 and the candidate isn't size-filtered out.
    fs::write(parent.join(name).join("placeholder"), b"x").unwrap();
}

fn scan_and_expect_rule(temp: &TempDir, expected_rule: &str) {
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

#[test]
fn rust_target_is_classified() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
    make_dir(temp.path(), "target");
    scan_and_expect_rule(&temp, "rust.target");
}

#[test]
fn go_vendor_is_classified() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("go.mod"), "module x\n").unwrap();
    make_dir(temp.path(), "vendor");
    scan_and_expect_rule(&temp, "go.vendor");
}

#[test]
fn java_maven_target_is_classified() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("pom.xml"), "<project/>").unwrap();
    make_dir(temp.path(), "target");
    scan_and_expect_rule(&temp, "java.maven_target");
}

#[test]
fn ios_pods_is_classified() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("Podfile"), "platform :ios").unwrap();
    make_dir(temp.path(), "Pods");
    scan_and_expect_rule(&temp, "ios.pods");
}

#[test]
fn python_pycache_is_classified() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("pyproject.toml"),
        "[project]\nname=\"x\"\n",
    )
    .unwrap();
    make_dir(temp.path(), "__pycache__");
    scan_and_expect_rule(&temp, "python.pycache");
}

#[test]
fn ruby_bundle_is_classified() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("Gemfile"), "source 'https://rubygems.org'").unwrap();
    make_dir(temp.path(), ".bundle");
    scan_and_expect_rule(&temp, "ruby.bundle");
}

#[test]
fn dotnet_bin_and_obj_are_classified() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("App.csproj"), "<Project/>").unwrap();
    make_dir(temp.path(), "bin");
    make_dir(temp.path(), "obj");

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
    .stdout(predicate::str::contains("\"ruleId\": \"dotnet.bin\""))
    .stdout(predicate::str::contains("\"ruleId\": \"dotnet.obj\""));
}

#[test]
fn generic_coverage_is_classified() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();
    make_dir(temp.path(), "coverage");
    scan_and_expect_rule(&temp, "generic.coverage");
}

#[test]
fn node_build_is_classified_with_caution_warning() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();
    make_dir(temp.path(), "build");

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
    .stdout(predicate::str::contains("\"ruleId\": \"node.build\""))
    .stdout(predicate::str::contains("\"safety\": \"caution\""));
}
