use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

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
        .stdout(predicate::str::contains("60.0%"))
        // The legend disambiguates the Safety and Risk columns (#356).
        .stdout(predicate::str::contains("Safety gates cleaning"))
        .stdout(predicate::str::contains(
            "Risk (0.00-0.85) is an independent advisory score",
        ));
}

/// The legend is human-only; `--json` output must stay pure.
#[test]
fn scan_json_has_no_table_legend() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    let assert = cmd
        .args([
            "scan",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        !stdout.contains("Safety gates cleaning"),
        "JSON output must not carry the human legend"
    );
    // And it is still valid JSON.
    serde_json::from_str::<serde_json::Value>(&stdout).expect("stdout must be pure JSON");
}

#[test]
fn clean_dry_run_does_not_delete() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();
    let audit_log = temp.path().join("audit.jsonl");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "clean",
        temp.path().to_str().unwrap(),
        "--all",
        "--dry-run",
        "--audit-log",
        audit_log.to_str().unwrap(),
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Plan:"));

    assert!(temp.path().join("node_modules").exists());
    assert!(!audit_log.exists(), "dry-run must not create audit log");
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
fn clean_permanent_yes_writes_audit_log() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    std::fs::write(temp.path().join("package.json"), "{}")?;
    std::fs::create_dir(temp.path().join("node_modules"))?;
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc")?;
    let audit_log = temp.path().join("logs").join("audit.jsonl");

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.args([
        "clean",
        temp.path().to_str().unwrap(),
        "--all",
        "--permanent",
        "--yes",
        "--audit-log",
        audit_log.to_str().unwrap(),
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Cleaned: 1"));

    let raw = std::fs::read_to_string(&audit_log)?;
    let lines = raw.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    let entry: Value = serde_json::from_str(lines[0])?;

    assert_eq!(entry["rule_id"], "node.node_modules");
    assert_eq!(entry["size_bytes"], 3);
    assert_eq!(entry["permanent"], true);
    assert_eq!(entry["mode"], "permanent");
    assert_eq!(entry["result"], "success");
    assert!(entry["reason"].is_null());
    assert!(entry["path"].as_str().unwrap().ends_with("node_modules"));
    assert!(entry["timestamp"].as_str().unwrap().contains('T'));
    Ok(())
}

#[test]
fn clean_rejects_audit_log_inside_selected_candidate() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    std::fs::write(temp.path().join("package.json"), "{}")?;
    let candidate = temp.path().join("node_modules");
    std::fs::create_dir(&candidate)?;
    std::fs::write(candidate.join("blob"), "abc")?;
    let audit_log = candidate.join("audit.jsonl");
    let root_arg = temp.path().to_string_lossy().into_owned();
    let audit_log_arg = audit_log.to_string_lossy().into_owned();

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.args([
        "clean",
        root_arg.as_str(),
        "--all",
        "--permanent",
        "--yes",
        "--audit-log",
        audit_log_arg.as_str(),
        "--min-size",
        "0",
    ])
    .assert()
    .failure()
    .stderr(
        predicate::str::contains("audit log").and(predicate::str::contains("selected candidate")),
    );

    assert!(
        candidate.exists(),
        "candidate must not be deleted after audit path rejection"
    );
    assert!(
        !audit_log.exists(),
        "audit log inside candidate must not be created"
    );
    Ok(())
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
fn duplicate_canonical_scan_roots_are_processed_once() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    std::fs::write(temp.path().join("package.json"), "{}")?;
    std::fs::create_dir(temp.path().join("node_modules"))?;
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc")?;
    let root = temp.path().to_string_lossy().into_owned();
    let alias = temp.path().join(".").to_string_lossy().into_owned();
    let plan = temp.path().join("plan.json");

    let mut scan = Command::cargo_bin("rclean")?;
    let output = scan
        .args([
            "scan",
            root.as_str(),
            root.as_str(),
            alias.as_str(),
            "--json",
            "--write-plan",
            plan.to_str().unwrap(),
            "--min-size",
            "0",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: Value = serde_json::from_slice(&output)?;

    assert_eq!(report["roots"].as_array().unwrap().len(), 1);
    assert_eq!(report["summary"]["projectsScanned"], 1);
    assert_eq!(report["summary"]["candidates"], 1);
    assert_eq!(report["projects"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["projects"][0]["candidates"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let plan_json: Value = serde_json::from_str(&std::fs::read_to_string(&plan)?)?;
    assert_eq!(plan_json["roots"].as_array().unwrap().len(), 1);
    assert_eq!(plan_json["projects"].as_array().unwrap().len(), 1);
    assert_eq!(plan_json["selected"].as_array().unwrap().len(), 1);

    let mut dry_run = Command::cargo_bin("rclean")?;
    dry_run
        .args(["clean", "--plan", plan.to_str().unwrap(), "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan: 1 candidates"));
    Ok(())
}

#[test]
fn distinct_canonical_scan_roots_are_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let child = temp.path().join("child");
    std::fs::create_dir(&child)?;
    std::fs::write(child.join("package.json"), "{}")?;
    std::fs::create_dir(child.join("node_modules"))?;
    std::fs::write(child.join("node_modules").join("blob"), "abc")?;
    let parent_arg = temp.path().to_string_lossy().into_owned();
    let child_arg = child.to_string_lossy().into_owned();

    let mut scan = Command::cargo_bin("rclean")?;
    let output = scan
        .args([
            "scan",
            child_arg.as_str(),
            parent_arg.as_str(),
            child_arg.as_str(),
            "--json",
            "--min-size",
            "0",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: Value = serde_json::from_slice(&output)?;
    let roots = report["roots"].as_array().unwrap();

    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0], child.canonicalize()?.to_string_lossy().as_ref());
    assert_eq!(
        roots[1],
        temp.path().canonicalize()?.to_string_lossy().as_ref()
    );
    Ok(())
}

#[test]
fn invalid_duplicate_scan_root_is_not_silently_dropped() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let root = temp.path().to_string_lossy().into_owned();
    let missing = temp.path().join("missing");
    let missing_arg = missing.to_string_lossy().into_owned();

    let mut scan = Command::cargo_bin("rclean")?;
    scan.args([
        "scan",
        root.as_str(),
        root.as_str(),
        missing_arg.as_str(),
        missing_arg.as_str(),
        "--json",
    ])
    .assert()
    .failure()
    .stderr(
        predicate::str::contains("cannot scan").and(predicate::str::contains(missing_arg.as_str())),
    );
    Ok(())
}

#[test]
fn clean_plan_uses_permanent_delete_mode_from_plan() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    std::fs::write(temp.path().join("package.json"), "{}")?;
    let candidate = temp.path().join("node_modules");
    std::fs::create_dir(&candidate)?;
    std::fs::write(candidate.join("blob"), "abc")?;
    let plan = temp.path().join("plan.json");

    let mut write_plan = Command::cargo_bin("rclean")?;
    write_plan
        .args([
            "clean",
            temp.path().to_str().unwrap(),
            "--all",
            "--dry-run",
            "--permanent",
            "--write-plan",
            plan.to_str().unwrap(),
            "--min-size",
            "0",
        ])
        .assert()
        .success();

    let mut dry_run = Command::cargo_bin("rclean")?;
    dry_run
        .args(["clean", "--plan", plan.to_str().unwrap(), "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("mode: permanent (dry run)"));
    assert!(candidate.exists(), "dry-run must not delete the candidate");

    let audit_log = temp.path().join("audit.jsonl");
    let mut replay = Command::cargo_bin("rclean")?;
    replay
        .args([
            "clean",
            "--plan",
            plan.to_str().unwrap(),
            "--yes",
            "--audit-log",
            audit_log.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleaned: 1"));

    assert!(
        !candidate.exists(),
        "permanent plan replay should remove the candidate"
    );
    let raw = std::fs::read_to_string(&audit_log)?;
    let entry: Value = serde_json::from_str(raw.lines().next().unwrap())?;
    assert_eq!(entry["mode"], "permanent");
    assert_eq!(entry["permanent"], true);
    Ok(())
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
        "dart.pub_hosted_cache",
        "dart.pub_git_cache",
        "dotnet.bin",
        "dotnet.obj",
        "ruby.bundle",
        "ruby.vendor_bundle",
        "generic.coverage",
        "homebrew.downloads",
        "android_sdk.download_intermediates",
        "android_sdk.legacy_build_cache",
        "jetbrains.system_caches",
        "jetbrains.logs",
        "android_studio.system_caches",
        "android_studio.logs",
        "ai.vllm_compile_cache",
        "ai.whisper_models",
        "ai.llama_cpp_cache",
        "ai.whisper_cpp_models",
        "ai.comfyui_models",
        "node.npm_transient",
        "agent.tmp_worktree",
        "ruby.bundle_compact_index",
        "cloud.kube_cache",
        "cloud.gcloud_logs",
        "editor.vscode_cache",
        "editor.cursor_cache",
        "editor.vscode_obsolete_extension",
        "editor.cursor_obsolete_extension",
        "claude.old_version",
        "app.electron_cache",
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
    // node.node_modules. explain now computes the same risk_score
    // the scan path emits per candidate, so the output should include
    // a `Risk: 0.??` line.
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();

    let candidate = temp.path().join("node_modules");

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "explain",
        "--activity-depth",
        "1",
        candidate.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Rule: node.node_modules"))
    .stdout(predicate::str::contains("Risk: 0."));
}

#[test]
fn explain_help_exposes_activity_depth() {
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["explain", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--activity-depth"));
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

#[test]
fn no_subcommand_without_tty_prints_help_and_exits_2() {
    // The no-arg default flow is interactive-only. Without a terminal
    // on stdin/stdout it must print help and never reach selection or
    // deletion, even inside a directory full of candidates.
    let temp = TempDir::new().unwrap();
    std::fs::create_dir_all(temp.path().join("app/node_modules/dep")).unwrap();
    std::fs::write(temp.path().join("app/package.json"), "{}").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.current_dir(temp.path())
        .assert()
        .code(2)
        .stdout(predicate::str::contains("Usage:"));

    assert!(temp.path().join("app/node_modules/dep").exists());
}

#[test]
fn clean_permanent_prints_not_recoverable_summary() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules/blob"), b"abc").unwrap();

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
    .stdout(predicate::str::contains("not recoverable"));
}

#[test]
fn clean_json_suppresses_recovery_summary() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules/blob"), b"abc").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "clean",
        temp.path().to_str().unwrap(),
        "--all",
        "--permanent",
        "--yes",
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("freed ").not());
}
