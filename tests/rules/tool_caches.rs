use super::common::make_dir;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

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
fn homebrew_downloads_is_classified_under_homebrew_cache() {
    let temp = TempDir::new().unwrap();
    let homebrew_cache = temp.path().join("Library").join("Caches").join("Homebrew");
    fs::create_dir_all(&homebrew_cache).unwrap();
    make_dir(&homebrew_cache, "downloads");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        homebrew_cache.to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "\"ruleId\": \"homebrew.downloads\"",
    ))
    .stdout(predicate::str::contains("\"safety\": \"safe\""))
    .stdout(predicate::str::contains("\"category\": \"cache\""));
}

#[test]
fn homebrew_downloads_outside_exact_anchor_is_not_classified() {
    let temp = TempDir::new().unwrap();
    let not_homebrew = temp
        .path()
        .join("Library")
        .join("Caches")
        .join("NotHomebrew");
    fs::create_dir_all(&not_homebrew).unwrap();
    make_dir(&not_homebrew, "downloads");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        not_homebrew.to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .code(3)
    .stdout(predicate::str::contains("\"ruleId\": \"homebrew.downloads\"").not());
}

#[test]
fn dart_pub_caches_are_classified_under_pub_cache() {
    let temp = TempDir::new().unwrap();
    let pub_cache = temp.path().join(".pub-cache");
    fs::create_dir_all(&pub_cache).unwrap();
    make_dir(&pub_cache, "hosted");
    make_dir(&pub_cache, "git");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        pub_cache.to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "\"ruleId\": \"dart.pub_hosted_cache\"",
    ))
    .stdout(predicate::str::contains(
        "\"ruleId\": \"dart.pub_git_cache\"",
    ))
    .stdout(predicate::str::contains("\"safety\": \"caution\""));
}

#[test]
fn dart_pub_cache_names_outside_exact_anchor_are_not_classified() {
    let temp = TempDir::new().unwrap();
    make_dir(temp.path(), "hosted");
    make_dir(temp.path(), "git");

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
    .stdout(predicate::str::contains("\"ruleId\": \"dart.pub_hosted_cache\"").not())
    .stdout(predicate::str::contains("\"ruleId\": \"dart.pub_git_cache\"").not());
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
