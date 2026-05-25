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
fn home_flag_expands_to_go_cache_roots_when_present() {
    let temp = TempDir::new().unwrap();
    let module_download = temp
        .path()
        .join("go")
        .join("pkg")
        .join("mod")
        .join("cache")
        .join("download");
    std::fs::create_dir_all(&module_download).unwrap();
    std::fs::write(module_download.join("blob"), "x").unwrap();

    #[cfg(target_os = "macos")]
    let build_cache = temp.path().join("Library").join("Caches").join("go-build");
    #[cfg(not(target_os = "macos"))]
    let build_cache = temp.path().join(".cache").join("go-build");
    std::fs::create_dir_all(&build_cache).unwrap();
    std::fs::write(build_cache.join("blob"), "x").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"ruleId\": \"go.module_download_cache\"",
        ))
        .stdout(predicate::str::contains("\"ruleId\": \"go.build_cache\""))
        .stdout(predicate::str::contains("\"safety\": \"safe\""));
}

#[test]
fn home_flag_expands_to_pnpm_cache_roots_when_present() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let legacy_store = temp.path().join(".pnpm-store").join("v3");
    std::fs::create_dir_all(&legacy_store)?;
    std::fs::write(legacy_store.join("blob"), "x")?;

    #[cfg(target_os = "macos")]
    let platform_store = temp.path().join("Library").join("pnpm").join("store");
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let platform_store = temp
        .path()
        .join(".local")
        .join("share")
        .join("pnpm")
        .join("store");
    #[cfg(target_os = "windows")]
    let platform_store = temp
        .path()
        .join("AppData")
        .join("Local")
        .join("pnpm")
        .join("store");
    std::fs::create_dir_all(&platform_store)?;
    std::fs::write(platform_store.join("blob"), "x")?;

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ruleId\": \"node.pnpm_store\""))
        .stdout(predicate::str::contains("\"safety\": \"safe\""));
    Ok(())
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
        .stdout(predicate::str::contains("go.module_download_cache"))
        .stdout(predicate::str::contains("node.pnpm_store"))
        .stdout(predicate::str::contains("xcode.derived_data"))
        .stdout(predicate::str::contains("of 18 rules applicable"));
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
fn agent_doctor_json_runs_for_codex() {
    let temp = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.env("HOME", temp.path())
        .env("TMPDIR", temp.path())
        .args(["agent", "doctor", "codex", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"tool\": \"codex\""))
        .stdout(predicate::str::contains("\"disk\""));
}

#[test]
fn agent_optimize_dry_run_prints_codex_update_commands() {
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["agent", "optimize", "codex", "--disable-auto-update"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Mode: dry-run"))
        .stdout(predicate::str::contains(
            "defaults write com.openai.codex SUAutomaticallyUpdate -bool false",
        ));
}

#[test]
fn agent_optimize_requires_an_action_flag() {
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["agent", "optimize", "codex"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "select at least one agent optimization flag",
        ));
}

#[cfg(target_os = "macos")]
#[test]
fn agent_optimize_yes_can_apply_to_sandbox_defaults_domain() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let domain = format!("com.openai.rclean-sandbox-{}-{suffix}", std::process::id());

    let _ = std::process::Command::new("defaults")
        .args(["delete", &domain])
        .output();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "agent",
        "optimize",
        "codex",
        "--disable-auto-update",
        "--yes",
        "--defaults-domain",
        &domain,
        "--json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"applied\": true"))
    .stdout(predicate::str::contains(&domain));

    let automatically_update = defaults_read(&domain, "SUAutomaticallyUpdate");
    let automatic_checks = defaults_read(&domain, "SUEnableAutomaticChecks");

    let _ = std::process::Command::new("defaults")
        .args(["delete", &domain])
        .output();

    assert_eq!(automatically_update.trim(), "0");
    assert_eq!(automatic_checks.trim(), "0");
}

#[cfg(target_os = "macos")]
fn defaults_read(domain: &str, key: &str) -> String {
    let output = std::process::Command::new("defaults")
        .args(["read", domain, key])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "defaults read failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
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
        .arg("tui")
        .arg(temp.path())
        .arg("--write-plan")
        .arg(&plan)
        .args(["--min-size", "0"])
        .write_stdin("a\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("wrote action plan"))
        .stderr(predicate::str::contains("falling back to text selection"));

    assert!(plan.exists());

    let mut clean = Command::cargo_bin("rclean").unwrap();
    clean
        .arg("clean")
        .arg("--plan")
        .arg(&plan)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan: 1 candidates"));
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
