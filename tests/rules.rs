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

#[test]
fn gradle_build_wins_over_node_build_in_mixed_project() {
    // Regression for the dispatch-order bug found in PR #28 review:
    // a project that has BOTH `package.json` and `build.gradle` should
    // classify `build/` as `java.gradle_build` (Safe), not as
    // `node.build` (Caution). The match-arm order in v0.1.0 put Gradle
    // first; the dispatch chain must replay that priority.
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();
    fs::write(temp.path().join("build.gradle"), "// gradle\n").unwrap();
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
    .stdout(predicate::str::contains(
        "\"ruleId\": \"java.gradle_build\"",
    ))
    .stdout(predicate::str::contains("\"safety\": \"safe\""));
}

#[test]
fn flutter_build_wins_over_node_build_in_mixed_project() {
    // Same regression in the Flutter+Node combo. v0.1.0 priority:
    // Gradle > Flutter > Node for the ambiguous `build/` name.
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();
    fs::write(temp.path().join("pubspec.yaml"), "name: x\n").unwrap();
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
    .stdout(predicate::str::contains("\"ruleId\": \"dart.build\""))
    .stdout(predicate::str::contains("\"safety\": \"safe\""));
}

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
fn cargo_registry_cache_is_classified_under_cargo_registry() {
    let temp = TempDir::new().unwrap();
    let registry = temp.path().join(".cargo").join("registry");
    fs::create_dir_all(&registry).unwrap();
    make_dir(&registry, "cache");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        registry.to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "\"ruleId\": \"cargo.registry_cache\"",
    ))
    .stdout(predicate::str::contains("\"safety\": \"safe\""))
    .stdout(predicate::str::contains("\"category\": \"cache\""));
}

#[test]
fn cargo_git_db_is_classified_under_cargo_git() {
    let temp = TempDir::new().unwrap();
    let git_dir = temp.path().join(".cargo").join("git");
    fs::create_dir_all(&git_dir).unwrap();
    make_dir(&git_dir, "db");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        git_dir.to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"ruleId\": \"cargo.git_db\""))
    .stdout(predicate::str::contains("\"safety\": \"safe\""));
}

#[test]
fn npm_cacache_is_classified_under_dot_npm() {
    // Synthesize <root>/.npm/_cacache
    let temp = TempDir::new().unwrap();
    let npm = temp.path().join(".npm");
    fs::create_dir_all(&npm).unwrap();
    make_dir(&npm, "_cacache");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["scan", npm.to_str().unwrap(), "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ruleId\": \"node.npm_cacache\""))
        .stdout(predicate::str::contains("\"safety\": \"safe\""))
        .stdout(predicate::str::contains("\"category\": \"cache\""));
}

#[test]
fn yarn_cache_is_classified_under_library_caches() {
    // Synthesize <root>/Library/Caches/Yarn
    let temp = TempDir::new().unwrap();
    let caches = temp.path().join("Library").join("Caches");
    fs::create_dir_all(&caches).unwrap();
    make_dir(&caches, "Yarn");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        caches.to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"ruleId\": \"node.yarn_cache\""))
    .stdout(predicate::str::contains("\"safety\": \"safe\""));
}

#[test]
fn pnpm_legacy_store_is_classified_under_dot_pnpm_store() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = TempDir::new()?;
    let pnpm_store = temp.path().join(".pnpm-store");
    fs::create_dir_all(&pnpm_store)?;
    let version_dir = pnpm_store.join("v3");
    fs::create_dir(&version_dir)?;
    fs::write(version_dir.join("placeholder"), b"x")?;

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.arg("scan")
        .arg(&pnpm_store)
        .args(["--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ruleId\": \"node.pnpm_store\""))
        .stdout(predicate::str::contains("\"safety\": \"safe\""))
        .stdout(predicate::str::contains("\"category\": \"cache\""));

    Ok(())
}

#[test]
fn pnpm_store_is_classified_under_platform_data_dir() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let pnpm_parent = temp.path().join("Library").join("pnpm");
    fs::create_dir_all(&pnpm_parent)?;
    let store = pnpm_parent.join("store");
    fs::create_dir(&store)?;
    fs::write(store.join("placeholder"), b"x")?;

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.arg("scan")
        .arg(&pnpm_parent)
        .args(["--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ruleId\": \"node.pnpm_store\""))
        .stdout(predicate::str::contains("\"safety\": \"safe\""));

    Ok(())
}

#[test]
fn pip_cache_is_classified_under_macos_library_caches() {
    let temp = TempDir::new().unwrap();
    let caches = temp.path().join("Library").join("Caches");
    fs::create_dir_all(&caches).unwrap();
    make_dir(&caches, "pip");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        caches.to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"ruleId\": \"pip.cache\""))
    .stdout(predicate::str::contains("\"safety\": \"safe\""))
    .stdout(predicate::str::contains("\"category\": \"cache\""));
}

#[test]
fn pip_cache_is_classified_under_xdg_cache() {
    let temp = TempDir::new().unwrap();
    let xdg = temp.path().join(".cache");
    fs::create_dir_all(&xdg).unwrap();
    make_dir(&xdg, "pip");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["scan", xdg.to_str().unwrap(), "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ruleId\": \"pip.cache\""))
        .stdout(predicate::str::contains("\"safety\": \"safe\""));
}

#[test]
fn gradle_caches_is_classified_under_dot_gradle() {
    let temp = TempDir::new().unwrap();
    let gradle = temp.path().join(".gradle");
    fs::create_dir_all(&gradle).unwrap();
    make_dir(&gradle, "caches");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        gradle.to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"ruleId\": \"gradle.caches\""))
    .stdout(predicate::str::contains("\"safety\": \"caution\""))
    .stdout(predicate::str::contains("\"category\": \"cache\""));
}

#[test]
fn maven_local_repo_is_classified_under_dot_m2() {
    let temp = TempDir::new().unwrap();
    let m2 = temp.path().join(".m2");
    fs::create_dir_all(&m2).unwrap();
    make_dir(&m2, "repository");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["scan", m2.to_str().unwrap(), "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ruleId\": \"maven.local_repo\""))
        .stdout(predicate::str::contains("\"safety\": \"caution\""))
        .stdout(predicate::str::contains("\"category\": \"cache\""));
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
fn cargo_cache_outside_cargo_registry_is_not_classified() {
    let temp = TempDir::new().unwrap();
    make_dir(temp.path(), "cache");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        temp.path().to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .code(3)
    .stdout(predicate::str::contains("\"ruleId\": \"cargo.registry_cache\"").not());
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
