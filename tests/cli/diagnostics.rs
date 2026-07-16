use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

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
        .stdout(predicate::str::contains("apple.idleassetsd"))
        .stdout(predicate::str::contains("of 59 rules applicable"));
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
fn scan_help_exposes_git_timeout() {
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["scan", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--git-timeout"));
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
