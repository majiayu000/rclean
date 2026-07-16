use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
#[cfg(target_os = "macos")]
use std::path::Path;
#[cfg(target_os = "macos")]
use std::process::Stdio;
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};
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
fn tmp_flag_conflicts_with_positional_paths_and_home() {
    let mut with_path = Command::cargo_bin("rclean").unwrap();
    with_path
        .args(["scan", "--tmp", "/tmp/somepath"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("cannot be used with")
                .or(predicate::str::contains("conflicts with")),
        );

    let mut with_home = Command::cargo_bin("rclean").unwrap();
    with_home
        .args(["scan", "--tmp", "--home"])
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
fn tmp_flag_runs_without_panicking_on_empty_tmp_root() {
    let temp = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.env("RCLEAN_TMP_ROOTS", temp.path())
        .args(["scan", "--tmp", "--json", "--min-size", "0"])
        .assert()
        .code(3);
}

#[test]
fn empty_scan_human_output_suggests_home_or_tmp() {
    let temp = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.arg("scan")
        .arg(temp.path())
        .assert()
        .code(3)
        .stdout(predicate::str::contains(
            "Hint: try `rclean scan --home` for toolchain caches or `rclean scan --tmp` for temp worktrees.",
        ));
}

#[test]
fn empty_scan_json_omits_home_tmp_hint() {
    let temp = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.arg("scan")
        .arg(temp.path())
        .arg("--json")
        .assert()
        .code(3)
        .stdout(predicate::str::contains("\"candidates\": 0"))
        .stdout(predicate::str::contains("rclean scan --home").not())
        .stdout(predicate::str::contains("rclean scan --tmp").not());
}

#[test]
fn tmp_flag_scans_rust_targets_under_temp_worktree() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let worktree = temp.path().join("remem-review");
    let target = worktree.join("target");
    std::fs::create_dir_all(&target)?;
    std::fs::write(
        worktree.join("Cargo.toml"),
        "[package]\nname = \"tmp-review\"\n",
    )?;
    std::fs::write(target.join("blob"), "x")?;

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("RCLEAN_TMP_ROOTS", temp.path())
        .args(["scan", "--tmp", "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ruleId\": \"rust.target\""))
        .stdout(predicate::str::contains("\"safety\": \"safe\""))
        .stdout(predicate::str::contains("remem-review"));
    Ok(())
}

#[test]
fn clean_tmp_all_dry_run_selects_temp_target() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let worktree = temp.path().join("rclean-review");
    let target = worktree.join("target");
    std::fs::create_dir_all(&target)?;
    std::fs::write(
        worktree.join("Cargo.toml"),
        "[package]\nname = \"tmp-review\"\n",
    )?;
    std::fs::write(target.join("blob"), "x")?;

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("RCLEAN_TMP_ROOTS", temp.path())
        .args(["clean", "--tmp", "--all", "--dry-run", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan: 1 candidates"));

    assert!(target.exists(), "dry-run must not delete the target dir");
    Ok(())
}

#[test]
fn clean_tmp_worktree_requires_include_caution() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let worktree = temp.path().join("rclean-whole-worktree");
    std::fs::create_dir(&worktree)?;
    std::fs::write(
        worktree.join("Cargo.toml"),
        "[package]\nname = \"tmp-whole-worktree\"\n",
    )?;
    std::fs::write(worktree.join("source.rs"), "fn main() {}\n")?;

    let mut default_clean = Command::cargo_bin("rclean")?;
    default_clean
        .env("RCLEAN_TMP_ROOTS", temp.path())
        .args(["clean", "--tmp", "--all", "--dry-run", "--min-size", "0"])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("rclean-whole-worktree"))
        .stdout(predicate::str::contains("caution"))
        .stdout(predicate::str::contains("Nothing selected."));

    let mut include_caution_clean = Command::cargo_bin("rclean")?;
    include_caution_clean
        .env("RCLEAN_TMP_ROOTS", temp.path())
        .args([
            "clean",
            "--tmp",
            "--all",
            "--include-caution",
            "--dry-run",
            "--min-size",
            "0",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan: 1 candidates"))
        .stdout(predicate::str::contains("agent.tmp_worktree"));

    assert!(worktree.exists(), "dry-run must not delete the worktree");
    Ok(())
}

#[test]
fn tmp_worktree_action_plan_revalidates() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let worktree = temp.path().join("rclean-plan-worktree");
    let plan = temp.path().join("plan.json");
    std::fs::create_dir(&worktree)?;
    std::fs::write(
        worktree.join("Cargo.toml"),
        "[package]\nname = \"tmp-plan-worktree\"\n",
    )?;
    std::fs::write(worktree.join("source.rs"), "fn main() {}\n")?;

    let mut scan = Command::cargo_bin("rclean")?;
    scan.env("RCLEAN_TMP_ROOTS", temp.path())
        .args([
            "scan",
            "--tmp",
            "--include-caution",
            "--write-plan",
            plan.to_str().unwrap(),
            "--min-size",
            "0",
        ])
        .assert()
        .success();

    let mut clean = Command::cargo_bin("rclean")?;
    clean
        .env("RCLEAN_TMP_ROOTS", temp.path())
        .args(["clean", "--plan", plan.to_str().unwrap(), "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan: 1 candidates"))
        .stdout(predicate::str::contains("agent.tmp_worktree"));

    assert!(worktree.exists(), "dry-run must not delete the worktree");
    Ok(())
}

#[test]
fn tmp_worktree_action_plan_rejects_non_tmp_root() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_root = TempDir::new()?;
    let worktree = tmp_root.path().join("rclean-plan-worktree");
    let plan = tmp_root.path().join("plan.json");
    std::fs::create_dir(&worktree)?;
    std::fs::write(
        worktree.join("Cargo.toml"),
        "[package]\nname = \"tmp-plan-worktree\"\n",
    )?;
    std::fs::write(worktree.join("source.rs"), "fn main() {}\n")?;

    let mut scan = Command::cargo_bin("rclean")?;
    scan.env("RCLEAN_TMP_ROOTS", tmp_root.path())
        .args([
            "scan",
            "--tmp",
            "--include-caution",
            "--write-plan",
            plan.to_str().unwrap(),
            "--min-size",
            "0",
        ])
        .assert()
        .success();

    let outside = TempDir::new()?;
    let outside_worktree = outside.path().join("rclean-plan-worktree");
    std::fs::create_dir(&outside_worktree)?;
    std::fs::write(
        outside_worktree.join("Cargo.toml"),
        "[package]\nname = \"outside-worktree\"\n",
    )?;
    std::fs::write(outside_worktree.join("source.rs"), "fn main() {}\n")?;

    let mut json: Value = serde_json::from_str(&std::fs::read_to_string(&plan)?)?;
    json["roots"] = Value::Array(vec![Value::String(outside.path().display().to_string())]);
    json["selected"][0]["path"] = Value::String(outside_worktree.display().to_string());
    std::fs::write(&plan, serde_json::to_string_pretty(&json)?)?;

    let mut clean = Command::cargo_bin("rclean")?;
    clean
        .env("RCLEAN_TMP_ROOTS", tmp_root.path())
        .args(["clean", "--plan", plan.to_str().unwrap(), "--dry-run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "is not recognized by any current rule",
        ));

    assert!(
        outside_worktree.exists(),
        "rejected tampered plan must not delete the outside worktree"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn clean_tmp_all_rejects_broad_rclean_tmp_roots_without_override() {
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.env("RCLEAN_TMP_ROOTS", "/")
        .args([
            "clean",
            "--tmp",
            "--all",
            "--dry-run",
            "--depth",
            "0",
            "--min-size",
            "0",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("broad root")
                .and(predicate::str::contains("--allow-broad-root")),
        );
}

#[cfg(unix)]
#[test]
fn clean_tmp_all_rejects_broad_tmpdir_without_override() {
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.env_remove("RCLEAN_TMP_ROOTS")
        .env("TMPDIR", "/")
        .args([
            "clean",
            "--tmp",
            "--all",
            "--dry-run",
            "--depth",
            "0",
            "--min-size",
            "0",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("broad root")
                .and(predicate::str::contains("--allow-broad-root")),
        );
}

#[cfg(target_os = "macos")]
#[test]
fn clean_tmp_permanent_refuses_rust_target_with_open_file() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = TempDir::new()?;
    let worktree = temp.path().join("rclean-open-target");
    let target = worktree.join("target");
    let blob = target.join("blob");
    std::fs::create_dir_all(&target)?;
    std::fs::write(
        worktree.join("Cargo.toml"),
        "[package]\nname = \"tmp-open-target\"\n",
    )?;
    std::fs::write(&blob, "x")?;

    let mut holder = std::process::Command::new("tail")
        .arg("-f")
        .arg(&blob)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if !wait_for_lsof_to_see_open_path(&target, holder.id())? {
        let _ = holder.kill();
        let _ = holder.wait();
        return Err(
            std::io::Error::other("test setup did not observe holder process with lsof").into(),
        );
    }

    let mut cmd = Command::cargo_bin("rclean")?;
    let output = cmd
        .env("RCLEAN_TMP_ROOTS", temp.path())
        .args([
            "clean",
            "--tmp",
            "--all",
            "--permanent",
            "--yes",
            "--min-size",
            "0",
        ])
        .output();

    let _ = holder.kill();
    let _ = holder.wait();
    let output = output?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "cleanup unexpectedly succeeded\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("Failed: 1"),
        "expected one failed candidate\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("files are open by process ids"),
        "expected open-file guard error\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        target.exists(),
        "open target must remain after validation rejection"
    );
    Ok(())
}

#[cfg(target_os = "macos")]
fn wait_for_lsof_to_see_open_path(
    path: &Path,
    pid: u32,
) -> Result<bool, Box<dyn std::error::Error>> {
    let expected = pid.to_string();
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        let output = std::process::Command::new("lsof")
            .args(["-t", "+D"])
            .arg(path)
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.split_whitespace().any(|raw| raw == expected) {
            return Ok(true);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(false)
}
