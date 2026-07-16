use assert_cmd::Command;
use predicates::prelude::*;
#[cfg(target_os = "macos")]
use serde_json::Value;
#[cfg(target_os = "macos")]
use std::path::Path;
use tempfile::TempDir;

#[cfg(target_os = "macos")]
fn make_non_empty_dir(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(path)?;
    std::fs::write(path.join("blob"), b"x")?;
    Ok(())
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
