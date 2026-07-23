use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn build_free_fixture(temp: &TempDir) {
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules/blob"), vec![0u8; 4096]).unwrap();
}

#[test]
fn free_target_met_writes_plan_and_exits_zero() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    build_free_fixture(&temp);
    let plan_path = temp.path().join("free-plan.json");

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.args([
        "free",
        "1kb",
        temp.path().to_str().unwrap(),
        "--min-size",
        "0",
        "--write-plan",
        plan_path.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Proposed set to free"))
    .stdout(predicate::str::contains("rclean clean --plan"));

    let plan: Value = serde_json::from_str(&std::fs::read_to_string(&plan_path)?)?;
    assert_eq!(plan["deleteMode"], "trash");
    assert!(
        !plan["selected"].as_array().unwrap().is_empty(),
        "plan must carry the proposed selection"
    );
    // free never deletes anything itself.
    assert!(temp.path().join("node_modules").exists());
    Ok(())
}

/// Regression for #349: `free` never deletes, so it must not leave a
/// plan file behind in whatever directory the user was standing in.
#[test]
fn free_default_plan_lands_in_state_dir_not_cwd() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    build_free_fixture(&temp);
    let cwd = TempDir::new()?;
    let state = TempDir::new()?;

    let mut cmd = Command::cargo_bin("rclean")?;
    let assert = cmd
        .current_dir(cwd.path())
        .env("XDG_STATE_HOME", state.path())
        .args([
            "free",
            "1kb",
            temp.path().to_str().unwrap(),
            "--min-size",
            "0",
        ])
        .assert()
        .success();

    // Nothing dropped into the working directory.
    let stray: Vec<_> = std::fs::read_dir(cwd.path())?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        stray.is_empty(),
        "free must leave the working directory clean, found: {stray:?}"
    );

    // The plan landed under $XDG_STATE_HOME/rclean/plans/ instead.
    let plans_dir = state.path().join("rclean").join("plans");
    let written: Vec<_> = std::fs::read_dir(&plans_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    assert_eq!(
        written.len(),
        1,
        "expected exactly one plan in {}",
        plans_dir.display()
    );

    // It is a real ActionPlan, and both printed lines name its
    // resolved path so the replay command is copy-pasteable.
    let plan: Value = serde_json::from_str(&std::fs::read_to_string(&written[0])?)?;
    assert_eq!(plan["deleteMode"], "trash");
    assert!(!plan["selected"].as_array().unwrap().is_empty());

    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let resolved = written[0].display().to_string();
    assert!(
        stdout.contains(&format!("wrote action plan: {resolved}")),
        "stdout must name the resolved plan path, got: {stdout}"
    );
    assert!(
        stdout.contains(&format!("rclean clean --plan {resolved}")),
        "replay hint must name the resolved plan path, got: {stdout}"
    );

    assert!(temp.path().join("node_modules").exists());
    Ok(())
}

/// A state directory that cannot be created is an error, never a
/// silent fall back to writing the plan into the working directory
/// (AGENTS.md: no silent degradation).
#[test]
fn free_reports_an_unusable_state_dir_instead_of_falling_back_to_cwd()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    build_free_fixture(&temp);
    let cwd = TempDir::new()?;
    let blocker = TempDir::new()?;

    // XDG_STATE_HOME points at a regular file, so create_dir_all for
    // `<file>/rclean/plans` cannot succeed.
    let file_path = blocker.path().join("not-a-directory");
    std::fs::write(&file_path, b"blocker")?;

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.current_dir(cwd.path())
        .env("XDG_STATE_HOME", &file_path)
        .args([
            "free",
            "1kb",
            temp.path().to_str().unwrap(),
            "--min-size",
            "0",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to create plan directory"));

    let stray: Vec<_> = std::fs::read_dir(cwd.path())?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        stray.is_empty(),
        "a failed state dir must not degrade into a cwd write, found: {stray:?}"
    );
    assert!(temp.path().join("node_modules").exists());
    Ok(())
}

/// `--write-plan` stays authoritative: the explicit destination is used
/// and the state directory is left untouched.
#[test]
fn free_write_plan_overrides_the_state_dir_default() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    build_free_fixture(&temp);
    let cwd = TempDir::new()?;
    let state = TempDir::new()?;
    let plan_path = temp.path().join("explicit-plan.json");

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.current_dir(cwd.path())
        .env("XDG_STATE_HOME", state.path())
        .args([
            "free",
            "1kb",
            temp.path().to_str().unwrap(),
            "--min-size",
            "0",
            "--write-plan",
            plan_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(plan_path.is_file(), "--write-plan must win");
    assert!(
        !state.path().join("rclean").join("plans").exists(),
        "--write-plan must not also populate the state directory"
    );
    Ok(())
}

#[test]
fn free_target_unmet_states_the_gap_and_exits_3() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    build_free_fixture(&temp);
    let plan_path = temp.path().join("free-plan.json");

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.args([
        "free",
        "100gb",
        temp.path().to_str().unwrap(),
        "--min-size",
        "0",
        "--write-plan",
        plan_path.to_str().unwrap(),
    ])
    .assert()
    .code(3)
    .stdout(predicate::str::contains("target not met"))
    .stdout(predicate::str::contains("short by"));

    assert!(temp.path().join("node_modules").exists());
    Ok(())
}

#[test]
fn free_with_no_candidates_exits_3() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    std::fs::write(temp.path().join("README.md"), "empty")?;

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.args([
        "free",
        "1gb",
        temp.path().to_str().unwrap(),
        "--min-size",
        "0",
    ])
    .assert()
    .code(3)
    .stdout(predicate::str::contains("no safe candidates"));
    Ok(())
}

#[test]
fn free_json_target_met_is_pure_and_links_the_written_plan()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    build_free_fixture(&temp);
    let plan_path = temp.path().join("free-json-plan.json");

    let mut cmd = Command::cargo_bin("rclean")?;
    let assert = cmd
        .args([
            "free",
            "1kb",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
            "--write-plan",
            plan_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let output: Value = serde_json::from_str(&stdout)?;
    let keys: Vec<_> = output
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            "candidates",
            "planPath",
            "schemaVersion",
            "selectedBytes",
            "targetBytes",
            "targetMet",
        ]
    );
    assert_eq!(output["schemaVersion"], 1);
    assert_eq!(output["targetBytes"], 1024);
    assert_eq!(output["targetMet"], true);
    assert_eq!(output["planPath"], plan_path.display().to_string());
    assert!(output["selectedBytes"].as_u64().unwrap() >= 1024);
    assert!(!stdout.contains("Proposed set"));
    assert!(!stdout.contains("review it"));

    let candidates = output["candidates"].as_array().unwrap();
    assert!(!candidates.is_empty());
    for field in [
        "path",
        "name",
        "ruleId",
        "category",
        "bytes",
        "safety",
        "requiresSudo",
        "reasons",
        "warnings",
        "restoreHint",
        "riskScore",
        "stalenessDays",
    ] {
        assert!(
            candidates[0].get(field).is_some(),
            "missing candidate field {field}"
        );
    }

    let plan: Value = serde_json::from_str(&std::fs::read_to_string(&plan_path)?)?;
    assert_eq!(output["candidates"][0]["path"], plan["selected"][0]["path"]);
    Ok(())
}

#[test]
fn free_json_shortfall_is_structured_and_preserves_exit_3() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = TempDir::new()?;
    build_free_fixture(&temp);
    let plan_path = temp.path().join("free-json-shortfall.json");

    let mut cmd = Command::cargo_bin("rclean")?;
    let assert = cmd
        .args([
            "free",
            "100gb",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
            "--write-plan",
            plan_path.to_str().unwrap(),
        ])
        .assert()
        .code(3);

    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let output: Value = serde_json::from_str(&stdout)?;
    assert_eq!(output["schemaVersion"], 1);
    assert_eq!(output["targetBytes"], 100 * 1024_u64.pow(3));
    assert_eq!(output["targetMet"], false);
    assert_eq!(output["planPath"], plan_path.display().to_string());
    assert!(output["selectedBytes"].as_u64().unwrap() > 0);
    assert!(!output["candidates"].as_array().unwrap().is_empty());
    assert!(!stdout.contains("target not met"));
    serde_json::from_str::<Value>(&std::fs::read_to_string(&plan_path)?)?;
    Ok(())
}

#[test]
fn free_json_no_candidates_has_null_plan_and_preserves_exit_3()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    std::fs::write(temp.path().join("README.md"), "empty")?;
    let plan_path = temp.path().join("must-not-exist.json");

    let mut cmd = Command::cargo_bin("rclean")?;
    let assert = cmd
        .args([
            "free",
            "1gb",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
            "--write-plan",
            plan_path.to_str().unwrap(),
        ])
        .assert()
        .code(3);

    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let output: Value = serde_json::from_str(&stdout)?;
    assert_eq!(output["schemaVersion"], 1);
    assert_eq!(output["targetBytes"], 1024_u64.pow(3));
    assert_eq!(output["selectedBytes"], 0);
    assert_eq!(output["targetMet"], false);
    assert!(output["planPath"].is_null());
    assert_eq!(output["candidates"], Value::Array(Vec::new()));
    assert!(!stdout.contains("no safe candidates"));
    assert!(!plan_path.exists());
    Ok(())
}

#[test]
fn free_json_zero_target_without_candidates_is_not_reported_as_met()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    std::fs::write(temp.path().join("README.md"), "empty")?;
    let plan_path = temp.path().join("zero-target-plan.json");

    let mut cmd = Command::cargo_bin("rclean")?;
    let assert = cmd
        .args([
            "free",
            "0",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
            "--write-plan",
            plan_path.to_str().unwrap(),
        ])
        .assert()
        .code(3);

    let output: Value = serde_json::from_slice(&assert.get_output().stdout)?;
    assert_eq!(output["targetBytes"], 0);
    assert_eq!(output["selectedBytes"], 0);
    assert_eq!(output["targetMet"], false);
    assert!(output["planPath"].is_null());
    assert_eq!(output["candidates"], Value::Array(Vec::new()));
    assert!(!plan_path.exists());
    Ok(())
}

#[test]
fn free_json_plan_write_failure_leaves_stdout_empty() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    build_free_fixture(&temp);
    let plan_path = temp.path().join("missing-parent/free-plan.json");

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.args([
        "free",
        "1kb",
        temp.path().to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
        "--write-plan",
        plan_path.to_str().unwrap(),
    ])
    .assert()
    .failure()
    .stdout(predicate::str::is_empty())
    .stderr(predicate::str::contains("plan io error"));

    assert!(!plan_path.exists());
    Ok(())
}

#[test]
fn free_interactive_requires_tty_and_deletes_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    build_free_fixture(&temp);

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.args([
        "free",
        "1kb",
        temp.path().to_str().unwrap(),
        "--interactive",
        "--min-size",
        "0",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "free --interactive requires an interactive terminal",
    ));

    assert!(temp.path().join("node_modules").exists());
    Ok(())
}

#[test]
fn free_interactive_rejects_json() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    build_free_fixture(&temp);

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.args([
        "free",
        "1kb",
        temp.path().to_str().unwrap(),
        "--interactive",
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "free --interactive cannot be combined with --json",
    ));

    assert!(temp.path().join("node_modules").exists());
    Ok(())
}

#[test]
fn scan_json_stdout_stays_pure_with_progress_forced_on() {
    // RCLEAN_PROGRESS=always exercises the progress reporter even
    // without a TTY; every progress byte must land on stderr so the
    // JSON contract on stdout is unaffected.
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules/blob"), b"abc").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    let assert = cmd
        .env("RCLEAN_PROGRESS", "always")
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
    let parsed: Value = serde_json::from_str(&stdout).expect("stdout must be pure JSON");
    assert!(parsed["summary"]["candidates"].as_u64().unwrap() >= 1);
    assert!(
        !stdout.contains("scanning:"),
        "progress must never reach stdout"
    );
}

#[test]
fn completions_generate_for_all_four_shells() {
    for shell in ["bash", "zsh", "fish", "powershell"] {
        let mut cmd = Command::cargo_bin("rclean").unwrap();
        let assert = cmd.args(["completions", shell]).assert().success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
        assert!(
            stdout.contains("rclean"),
            "{shell} completions must mention the binary"
        );
    }
}

#[test]
fn man_page_renders_roff() {
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["man"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".TH rclean"));
}
