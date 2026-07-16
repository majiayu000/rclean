use super::common::{make_dir, scan_and_expect_rule};
use assert_cmd::Command;
use predicates::prelude::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn scan_rule_safety(root: &Path, rule_prefix: &str) -> BTreeMap<String, String> {
    let output = Command::cargo_bin("rclean")
        .unwrap()
        .args([
            "scan",
            root.to_str().expect("fixture root should be UTF-8"),
            "--json",
            "--min-size",
            "0",
            "--include-blocked",
        ])
        .output()
        .unwrap();
    let exit_code = output.status.code();
    assert!(
        matches!(exit_code, Some(0 | 3)),
        "scan exited with unexpected status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("scan stdout should be valid JSON");
    let candidate_count = report["summary"]["candidates"]
        .as_u64()
        .expect("summary.candidates should be an unsigned integer");
    let expected_exit_code = if candidate_count == 0 { 3 } else { 0 };
    assert_eq!(
        exit_code,
        Some(expected_exit_code),
        "scan exit status should match summary.candidates={candidate_count}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let projects = report["projects"]
        .as_array()
        .expect("projects should be an array");
    let mut rules = BTreeMap::new();
    for project in projects {
        let candidates = project["candidates"]
            .as_array()
            .expect("candidates should be an array");
        for candidate in candidates {
            let rule_id = candidate["ruleId"]
                .as_str()
                .expect("candidate ruleId should be a string");
            if !rule_id.starts_with(rule_prefix) {
                continue;
            }
            let safety = candidate["safety"]
                .as_str()
                .expect("candidate safety should be a string");
            assert!(
                rules
                    .insert(rule_id.to_string(), safety.to_string())
                    .is_none(),
                "duplicate rule id {rule_id}"
            );
        }
    }
    rules
}

fn expected_rule_safety(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(rule_id, safety)| ((*rule_id).to_string(), (*safety).to_string()))
        .collect()
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
fn node_classifier_matrix_is_exact() {
    let node = TempDir::new().unwrap();
    fs::write(node.path().join("package.json"), "{}").unwrap();
    for name in [
        "node_modules",
        ".next",
        ".turbo",
        ".vite",
        ".parcel-cache",
        "build",
        "dist",
        "out",
    ] {
        make_dir(node.path(), name);
    }

    assert_eq!(
        scan_rule_safety(node.path(), "node."),
        expected_rule_safety(&[
            ("node.build", "caution"),
            ("node.dist", "caution"),
            ("node.next", "safe"),
            ("node.node_modules", "safe"),
            ("node.out", "caution"),
            ("node.parcel", "safe"),
            ("node.turbo", "safe"),
            ("node.vite", "safe"),
        ])
    );

    let no_node_marker = TempDir::new().unwrap();
    fs::write(
        no_node_marker.path().join("Cargo.toml"),
        "[package]\nname='not-node'\nversion='0.1.0'\n",
    )
    .unwrap();
    for name in [
        "node_modules",
        ".next",
        ".turbo",
        ".vite",
        ".parcel-cache",
        "build",
        "dist",
        "out",
    ] {
        make_dir(no_node_marker.path(), name);
    }
    assert!(scan_rule_safety(no_node_marker.path(), "node.").is_empty());
}

#[test]
fn python_classifier_matrix_is_exact() {
    let python = TempDir::new().unwrap();
    fs::write(
        python.path().join("pyproject.toml"),
        "[project]\nname='example'\n",
    )
    .unwrap();
    for name in [
        ".venv",
        "venv",
        "__pycache__",
        ".pytest_cache",
        ".mypy_cache",
        ".ruff_cache",
        ".tox",
    ] {
        make_dir(python.path(), name);
    }
    fs::write(
        python.path().join(".venv").join("pyvenv.cfg"),
        "home = test",
    )
    .unwrap();
    fs::write(python.path().join("venv").join("pyvenv.cfg"), "home = test").unwrap();

    assert_eq!(
        scan_rule_safety(python.path(), "python."),
        expected_rule_safety(&[
            ("python.mypy", "safe"),
            ("python.pycache", "safe"),
            ("python.pytest", "safe"),
            ("python.ruff", "safe"),
            ("python.tox", "caution"),
            ("python.venv_dot", "safe"),
            ("python.venv_plain", "safe"),
        ])
    );

    let invalid_venvs = TempDir::new().unwrap();
    fs::write(
        invalid_venvs.path().join("pyproject.toml"),
        "[project]\nname='invalid-venvs'\n",
    )
    .unwrap();
    make_dir(invalid_venvs.path(), ".venv");
    make_dir(invalid_venvs.path(), "venv");
    assert_eq!(
        scan_rule_safety(invalid_venvs.path(), "python."),
        expected_rule_safety(&[("python.venv_plain", "blocked")])
    );

    let no_python_marker = TempDir::new().unwrap();
    fs::write(
        no_python_marker.path().join("Cargo.toml"),
        "[package]\nname='not-python'\nversion='0.1.0'\n",
    )
    .unwrap();
    for name in [
        ".venv",
        "venv",
        "__pycache__",
        ".pytest_cache",
        ".mypy_cache",
        ".ruff_cache",
        ".tox",
    ] {
        make_dir(no_python_marker.path(), name);
    }
    assert!(scan_rule_safety(no_python_marker.path(), "python.").is_empty());
}
