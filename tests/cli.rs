use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn home_flag_conflicts_with_positional_paths() {
    // --home is mutually exclusive with positional paths
    // (clap-enforced via conflicts_with = "paths").
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["scan", "--home", "/tmp/somepath"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("cannot be used with")
                .or(predicate::str::contains("conflicts with")),
        );
}

#[test]
fn home_flag_runs_without_panicking_on_empty_home() {
    // With HOME pointed at a temp dir containing none of the
    // toolchain dirs, --home should still exit cleanly (just with
    // exit code 3 = no candidates) rather than panic or error.
    let temp = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .code(3); // no candidates because no toolchain dirs exist
}

#[test]
fn home_flag_expands_to_cargo_root_when_present() {
    // With a synthetic ~/.cargo/registry/cache, --home should pick
    // it up via the cargo.registry_cache rule, proving the path
    // expansion + rule dispatch work end-to-end.
    let temp = TempDir::new().unwrap();
    let registry = temp.path().join(".cargo").join("registry");
    std::fs::create_dir_all(&registry).unwrap();
    std::fs::create_dir(registry.join("cache")).unwrap();
    std::fs::write(registry.join("cache").join("blob"), "x").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"ruleId\": \"cargo.registry_cache\"",
        ))
        .stdout(predicate::str::contains("\"safety\": \"safe\""));
}

#[test]
fn doctor_prints_rule_status_table() {
    // Run with a clean HOME so the output is deterministic
    // (no rules applicable).
    let temp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.env("HOME", temp.path())
        .arg("doctor")
        .assert()
        .code(3) // 0 rules applicable → exit 3
        .stdout(predicate::str::contains("cargo.registry_cache"))
        .stdout(predicate::str::contains("xcode.derived_data"))
        .stdout(predicate::str::contains("of 9 rules applicable"));
}

#[test]
fn doctor_marks_existing_anchor_applicable() {
    // Synthesize ~/.cargo/registry so cargo.registry_cache applies.
    let temp = TempDir::new().unwrap();
    std::fs::create_dir_all(temp.path().join(".cargo").join("registry")).unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.env("HOME", temp.path())
        .arg("doctor")
        .assert()
        .success() // ≥1 applicable → exit 0
        .stdout(predicate::str::contains("cargo.registry_cache"))
        .stdout(predicate::str::contains("applicable"));
}

#[test]
fn help_prints_usage() {
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Find and clean rebuildable"));
}

#[test]
fn watch_help_exposes_poll_interval() {
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["watch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--every"));
}

#[cfg(feature = "tui")]
#[test]
fn tui_falls_back_to_text_selection_when_alt_screen_unavailable() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();
    let plan = temp.path().join("tui-plan.json");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.env("TERM", "dumb")
        .args([
            "tui",
            temp.path().to_str().unwrap(),
            "--write-plan",
            plan.to_str().unwrap(),
            "--min-size",
            "0",
        ])
        .write_stdin("a\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("wrote action plan"))
        .stderr(predicate::str::contains("falling back to text selection"));

    assert!(plan.exists());
}

#[test]
fn scan_json_detects_node_modules() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();

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
        "\"ruleId\": \"node.node_modules\"",
    ))
    .stdout(predicate::str::contains("\"projectBytes\": 5"))
    .stdout(predicate::str::contains("\"artifactPercent\": 60.0"));
}

#[test]
fn scan_table_shows_biggest_wins_and_junk_percent() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["scan", temp.path().to_str().unwrap(), "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Biggest wins:"))
        .stdout(predicate::str::contains("Junk"))
        .stdout(predicate::str::contains("60.0%"));
}

#[test]
fn clean_dry_run_does_not_delete() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "clean",
        temp.path().to_str().unwrap(),
        "--all",
        "--dry-run",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Plan:"));

    assert!(temp.path().join("node_modules").exists());
}

#[test]
fn clean_permanent_yes_deletes_safe_candidate() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "clean",
        temp.path().to_str().unwrap(),
        "--all",
        "--permanent",
        "--yes",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Cleaned: 1"));

    assert!(!temp.path().join("node_modules").exists());
}

#[test]
fn scan_write_plan_then_clean_plan_dry_run() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();
    let plan = temp.path().join("plan.json");

    let mut scan = Command::cargo_bin("rclean").unwrap();
    scan.args([
        "scan",
        temp.path().to_str().unwrap(),
        "--write-plan",
        plan.to_str().unwrap(),
        "--min-size",
        "0",
    ])
    .assert()
    .success();

    assert!(plan.exists());

    let mut clean = Command::cargo_bin("rclean").unwrap();
    clean
        .args(["clean", "--plan", plan.to_str().unwrap(), "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan: 1 candidates"));

    assert!(temp.path().join("node_modules").exists());
}

#[test]
fn ruby_vendor_bundle_plan_dry_run_replays_successfully() {
    let temp = TempDir::new().unwrap();
    std::fs::write(
        temp.path().join("Gemfile"),
        "source 'https://rubygems.org'\n",
    )
    .unwrap();
    std::fs::create_dir_all(temp.path().join("vendor").join("bundle")).unwrap();
    std::fs::write(
        temp.path().join("vendor").join("bundle").join("cache.txt"),
        "abc",
    )
    .unwrap();
    let plan = temp.path().join("plan.json");

    let mut scan = Command::cargo_bin("rclean").unwrap();
    scan.args([
        "scan",
        temp.path().to_str().unwrap(),
        "--write-plan",
        plan.to_str().unwrap(),
        "--min-size",
        "0",
        "--include-caution",
    ])
    .assert()
    .success();

    assert!(plan.exists());

    let mut clean = Command::cargo_bin("rclean").unwrap();
    clean
        .args(["clean", "--plan", plan.to_str().unwrap(), "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan: 1 candidates"));

    assert!(temp.path().join("vendor").join("bundle").exists());
}

#[test]
fn rules_lists_every_classifier_emitted_id() {
    // Guards against the catalog/classifier drift where rule_ids like
    // node.build / node.dist / node.out were emitted by classify_candidate
    // but missing from `rclean rules` output.
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    let output = cmd.arg("rules").assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    let expected = [
        "node.node_modules",
        "node.next",
        "node.turbo",
        "node.vite",
        "node.parcel",
        "node.build",
        "node.dist",
        "node.out",
        "python.venv_dot",
        "python.venv_plain",
        "python.pycache",
        "python.pytest",
        "python.mypy",
        "python.ruff",
        "python.tox",
        "rust.target",
        "go.vendor",
        "ios.pods",
        "java.maven_target",
        "java.gradle_build",
        "java.gradle_cache_local",
        "dart.build",
        "dart.tool",
        "dotnet.bin",
        "dotnet.obj",
        "ruby.bundle",
        "ruby.vendor_bundle",
        "generic.coverage",
    ];
    let missing: Vec<&&str> = expected
        .iter()
        .filter(|id| !stdout.contains(**id))
        .collect();
    assert!(
        missing.is_empty(),
        "rule_ids emitted by classifier but missing from `rclean rules` output: {missing:?}"
    );
}

#[test]
fn clean_interactive_selection_accepts_number() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "clean",
        temp.path().to_str().unwrap(),
        "--dry-run",
        "--min-size",
        "0",
    ])
    .write_stdin("1\n")
    .assert()
    .success()
    .stdout(predicate::str::contains("Select candidates"))
    .stdout(predicate::str::contains("Project:"))
    .stdout(predicate::str::contains("package.json marker found"))
    .stdout(predicate::str::contains("Plan: 1 candidates"));

    assert!(temp.path().join("node_modules").exists());
}

#[test]
fn explain_emits_risk_score_for_matched_candidate() {
    // A node_modules under a real package.json project should match
    // node.node_modules. explain_path now computes the same risk_score
    // the scan path emits per candidate, so the output should include
    // a `Risk: 0.??` line.
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();

    let candidate = temp.path().join("node_modules");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["explain", candidate.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rule: node.node_modules"))
        .stdout(predicate::str::contains("Risk: 0."));
}

#[test]
fn explain_skips_risk_score_for_unmatched_path() {
    // A path that doesn't match any built-in rule should report
    // Safety::Unknown and omit the Risk line — risk_score is None
    // when there's no project context to score against.
    let temp = TempDir::new().unwrap();
    let stray = temp.path().join("not_a_candidate_name");
    std::fs::create_dir(&stray).unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["explain", stray.to_str().unwrap()])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("Safety: unknown"))
        .stdout(predicate::str::contains("Risk:").not());
}
