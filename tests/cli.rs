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

#[cfg(target_os = "macos")]
fn make_non_empty_dir(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(path)?;
    std::fs::write(path.join("blob"), b"x")?;
    Ok(())
}

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
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let build_cache = temp.path().join(".cache").join("go-build");
    #[cfg(target_os = "windows")]
    let build_cache = temp.path().join("AppData").join("Local").join("go-build");
    std::fs::create_dir_all(&build_cache).unwrap();
    std::fs::write(build_cache.join("blob"), "x").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ruleId\": \"go.module_cache\""))
        .stdout(predicate::str::contains("\"ruleId\": \"go.build_cache\""))
        .stdout(predicate::str::contains("\"safety\": \"caution\""))
        .stdout(predicate::str::contains("\"ruleId\": \"go.module_download_cache\"").not());
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

#[cfg(target_os = "macos")]
#[test]
fn home_flag_reports_global_app_cache_rules() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let caches = temp.path().join("Library").join("Caches");
    let app_support_google = temp
        .path()
        .join("Library")
        .join("Application Support")
        .join("Google");

    let playwright = caches.join("ms-playwright");
    let shipit = caches.join("com.microsoft.VSCode.ShipIt");
    let chrome_cache = caches.join("Google").join("Chrome");
    let google_updater = app_support_google.join("GoogleUpdater");
    let chrome_profile = app_support_google.join("Chrome");

    make_non_empty_dir(&playwright)?;
    make_non_empty_dir(&shipit)?;
    make_non_empty_dir(&chrome_cache)?;
    make_non_empty_dir(&google_updater)?;
    make_non_empty_dir(&chrome_profile)?;

    let mut scan = Command::cargo_bin("rclean")?;
    let scan_output = scan
        .env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: Value = serde_json::from_slice(&scan_output)?;
    let candidates: Vec<&Value> = report["projects"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|project| project["candidates"].as_array().into_iter().flatten())
        .collect();

    for (rule_id, path) in [
        ("playwright.browsers", playwright),
        ("app.shipit_caches", shipit),
        ("chrome.cache", chrome_cache),
        ("chrome.google_updater", google_updater),
    ] {
        let path = std::fs::canonicalize(path)?;
        let path = path.display().to_string();
        let scan_candidate = candidates.iter().find(|candidate| {
            candidate["ruleId"].as_str() == Some(rule_id)
                && candidate["path"].as_str() == Some(path.as_str())
        });
        assert!(
            scan_candidate.is_some(),
            "scan --home should report {rule_id} at {path}; candidates: {candidates:#?}"
        );
        assert_eq!(
            scan_candidate.and_then(|candidate| candidate["safety"].as_str()),
            Some("safe"),
            "scan --home should report {rule_id} as safe"
        );

        let mut explain = Command::cargo_bin("rclean")?;
        let explain_output = explain
            .arg("explain")
            .arg(&path)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let explain_output = String::from_utf8(explain_output)?;
        assert!(explain_output.contains(&format!("Rule: {rule_id}")));
        assert!(explain_output.contains("Safety: safe"));
    }

    let chrome_profile = std::fs::canonicalize(chrome_profile)?.display().to_string();
    assert!(
        !candidates.iter().any(|candidate| {
            candidate["ruleId"].as_str() == Some("chrome.cache")
                && candidate["path"].as_str() == Some(chrome_profile.as_str())
        }),
        "Application Support/Google/Chrome user data must not be classified as chrome.cache"
    );

    Ok(())
}

#[test]
fn home_flag_expands_to_bun_cache_not_runtime_root() -> Result<(), Box<dyn std::error::Error>> {
    // Issue #103 safety invariant: the rule MUST target
    // ~/.bun/install/cache, NEVER ~/.bun itself (which holds the
    // Bun runtime binary). This test enforces both halves.
    let temp = TempDir::new()?;
    let install_cache = temp.path().join(".bun").join("install").join("cache");
    std::fs::create_dir_all(&install_cache)?;
    std::fs::write(install_cache.join("blob"), "x")?;
    // Synthesize a Bun runtime binary alongside install/ to prove
    // it stays untouched.
    let bin = temp.path().join(".bun").join("bin");
    std::fs::create_dir_all(&bin)?;
    std::fs::write(bin.join("bun"), "fake binary")?;

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        // install/cache is matched.
        .stdout(predicate::str::contains("\"ruleId\": \"bun.cache\""))
        .stdout(predicate::str::contains("\"safety\": \"safe\""))
        // The Bun runtime root must NOT appear as a candidate path.
        .stdout(predicate::str::contains("/.bun\",").not())
        .stdout(predicate::str::contains("/.bun/bin").not());
    Ok(())
}

#[test]
fn home_flag_reports_xdg_browser_and_lint_caches() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    for name in ["puppeteer", "pre-commit"] {
        let path = temp.path().join(".cache").join(name);
        std::fs::create_dir_all(&path)?;
        std::fs::write(path.join("blob"), "x")?;
    }

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"ruleId\": \"browser.puppeteer\"",
        ))
        .stdout(predicate::str::contains("\"safety\": \"caution\""))
        .stdout(predicate::str::contains("\"ruleId\": \"pre_commit.cache\""))
        .stdout(predicate::str::contains("\"safety\": \"safe\""));
    Ok(())
}

#[test]
fn home_flag_reports_ollama_models_as_report_only_never_selected()
-> Result<(), Box<dyn std::error::Error>> {
    // Issue #102 safety invariant: ~/.ollama/models is user data,
    // not cache. It must be reported (so the user sees the size)
    // but never selected for cleanup, even with --include-blocked.
    let temp = TempDir::new()?;
    let models = temp.path().join(".ollama").join("models");
    std::fs::create_dir_all(&models)?;
    std::fs::write(models.join("manifest.json"), "x")?;

    // 1. Plain scan: must report the path with report-only safety.
    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ruleId\": \"ai.ollama_models\""))
        .stdout(predicate::str::contains("\"safety\": \"report-only\""));

    // 2. clean --all --include-caution --include-blocked must NOT
    //    select the Ollama path. The plan must come back empty.
    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", temp.path())
        .args([
            "clean",
            "--home",
            "--all",
            "--include-caution",
            "--include-blocked",
            "--dry-run",
            "--min-size",
            "0",
        ])
        .assert()
        // Exit code 3 = no candidates selected (because Ollama is
        // ReportOnly and there's nothing else under the synthetic
        // home).
        .code(3);
    Ok(())
}

#[test]
fn home_flag_reports_llama_cpp_cache_as_report_only_never_selected()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let models = temp.path().join(".cache").join("llama.cpp");
    std::fs::create_dir_all(&models)?;
    std::fs::write(models.join("model.gguf"), "x")?;

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"ruleId\": \"ai.llama_cpp_cache\"",
        ))
        .stdout(predicate::str::contains("\"safety\": \"report-only\""));

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", temp.path())
        .args([
            "clean",
            "--home",
            "--all",
            "--include-caution",
            "--include-blocked",
            "--dry-run",
            "--min-size",
            "0",
        ])
        .assert()
        .code(3);
    Ok(())
}

#[test]
fn home_flag_reports_homebrew_dart_vllm_and_whisper_caches()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    for path in [
        temp.path()
            .join(".cache")
            .join("Homebrew")
            .join("downloads"),
        temp.path().join(".pub-cache").join("hosted"),
        temp.path().join(".pub-cache").join("git"),
        temp.path()
            .join(".cache")
            .join("vllm")
            .join("torch_compile_cache"),
        temp.path().join(".cache").join("whisper"),
    ] {
        std::fs::create_dir_all(&path)?;
        std::fs::write(path.join("blob"), "x")?;
    }

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"ruleId\": \"homebrew.downloads\"",
        ))
        .stdout(predicate::str::contains(
            "\"ruleId\": \"dart.pub_hosted_cache\"",
        ))
        .stdout(predicate::str::contains(
            "\"ruleId\": \"dart.pub_git_cache\"",
        ))
        .stdout(predicate::str::contains(
            "\"ruleId\": \"ai.vllm_compile_cache\"",
        ))
        .stdout(predicate::str::contains(
            "\"ruleId\": \"ai.whisper_models\"",
        ));
    Ok(())
}

#[test]
fn home_flag_reports_user_tool_safe_caches() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    for path in [
        temp.path().join(".npm").join("_npx"),
        temp.path().join(".npm").join("_logs"),
        temp.path().join(".npm").join("_prebuilds"),
        temp.path()
            .join(".bundle")
            .join("cache")
            .join("compact_index"),
        temp.path().join(".kube").join("cache"),
        temp.path().join(".config").join("gcloud").join("logs"),
    ] {
        std::fs::create_dir_all(&path)?;
        std::fs::write(path.join("blob"), "x")?;
    }

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"ruleId\": \"node.npm_transient\"",
        ))
        .stdout(predicate::str::contains(
            "\"ruleId\": \"ruby.bundle_compact_index\"",
        ))
        .stdout(predicate::str::contains("\"ruleId\": \"cloud.kube_cache\""))
        .stdout(predicate::str::contains(
            "\"ruleId\": \"cloud.gcloud_logs\"",
        ))
        .stdout(predicate::str::contains("\"safety\": \"safe\""));
    Ok(())
}

#[test]
fn home_flag_reports_obsolete_editor_and_claude_versions_as_caution()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    for path in [
        temp.path()
            .join(".vscode")
            .join("extensions")
            .join("publisher.tool-1.0.0"),
        temp.path()
            .join(".vscode")
            .join("extensions")
            .join("publisher.tool-1.1.0"),
        temp.path()
            .join(".local")
            .join("share")
            .join("claude")
            .join("versions")
            .join("1.0.0"),
        temp.path()
            .join(".local")
            .join("share")
            .join("claude")
            .join("versions")
            .join("1.1.0"),
    ] {
        std::fs::create_dir_all(&path)?;
        std::fs::write(path.join("blob"), "x")?;
    }

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"ruleId\": \"editor.vscode_obsolete_extension\"",
        ))
        .stdout(predicate::str::contains(
            "\"ruleId\": \"claude.old_version\"",
        ))
        .stdout(predicate::str::contains("\"safety\": \"caution\""));
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn home_flag_reports_macos_editor_app_caches_without_user_state()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let code = temp
        .path()
        .join("Library")
        .join("Application Support")
        .join("Code");
    let cursor = temp
        .path()
        .join("Library")
        .join("Application Support")
        .join("Cursor");
    let notion = temp
        .path()
        .join("Library")
        .join("Application Support")
        .join("Notion");
    for path in [
        code.join("logs"),
        code.join("Cache"),
        code.join("User"),
        code.join("globalStorage"),
        cursor.join("CachedData"),
        cursor.join("workspaceStorage"),
        notion.join("GPUCache"),
        notion.join("Partitions"),
    ] {
        make_non_empty_dir(&path)?;
    }

    let mut cmd = Command::cargo_bin("rclean")?;
    let output = cmd
        .env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output)?;
    assert!(stdout.contains("\"ruleId\": \"editor.vscode_cache\""));
    assert!(stdout.contains("\"ruleId\": \"editor.cursor_cache\""));
    assert!(stdout.contains("\"ruleId\": \"app.electron_cache\""));
    assert!(stdout.contains("\"safety\": \"caution\""));
    assert!(!stdout.contains("/User\""));
    assert!(!stdout.contains("/globalStorage\""));
    assert!(!stdout.contains("/workspaceStorage\""));
    assert!(!stdout.contains("/Partitions\""));
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
