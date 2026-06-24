use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;
use tempfile::TempDir;

fn make_non_empty_dir(path: &Path) {
    std::fs::create_dir_all(path).unwrap();
    std::fs::write(path.join("blob"), b"x").unwrap();
}

#[test]
fn ide_caches_and_logs_are_classified_under_exact_vendor_anchors() {
    let temp = TempDir::new().unwrap();
    for path in [
        temp.path()
            .join("Library")
            .join("Caches")
            .join("JetBrains")
            .join("IntelliJIdea2024.3")
            .join("caches"),
        temp.path()
            .join("Library")
            .join("Logs")
            .join("JetBrains")
            .join("IntelliJIdea2024.3"),
        temp.path()
            .join(".cache")
            .join("JetBrains")
            .join("PyCharmCE2025.1")
            .join("log"),
        temp.path()
            .join("Library")
            .join("Caches")
            .join("Google")
            .join("AndroidStudio2024.3")
            .join("caches"),
        temp.path()
            .join("Library")
            .join("Logs")
            .join("Google")
            .join("AndroidStudio2024.3"),
        temp.path()
            .join(".cache")
            .join("Google")
            .join("AndroidStudioPreview2025.1")
            .join("log"),
    ] {
        make_non_empty_dir(&path);
    }

    let roots = [
        temp.path().join("Library").join("Caches").join("JetBrains"),
        temp.path().join("Library").join("Logs").join("JetBrains"),
        temp.path().join(".cache"),
        temp.path().join("Library").join("Caches").join("Google"),
        temp.path().join("Library").join("Logs").join("Google"),
    ];
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.arg("scan");
    for root in roots {
        cmd.arg(root);
    }
    cmd.args(["--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"ruleId\": \"jetbrains.system_caches\"",
        ))
        .stdout(predicate::str::contains("\"ruleId\": \"jetbrains.logs\""))
        .stdout(predicate::str::contains(
            "\"ruleId\": \"android_studio.system_caches\"",
        ))
        .stdout(predicate::str::contains(
            "\"ruleId\": \"android_studio.logs\"",
        ))
        .stdout(predicate::str::contains("\"safety\": \"caution\""))
        .stdout(predicate::str::contains("Close the IDE"));
}

#[test]
fn ide_rules_reject_config_plugins_history_projects_sdks_and_avds() {
    let temp = TempDir::new().unwrap();
    for path in [
        temp.path()
            .join("Library")
            .join("Application Support")
            .join("JetBrains")
            .join("IntelliJIdea2024.3")
            .join("caches"),
        temp.path()
            .join(".config")
            .join("JetBrains")
            .join("PyCharm2024.3")
            .join("caches"),
        temp.path()
            .join(".local")
            .join("share")
            .join("JetBrains")
            .join("PyCharm2024.3")
            .join("log"),
        temp.path()
            .join("Library")
            .join("Caches")
            .join("JetBrains")
            .join("IntelliJIdea2024.3")
            .join("plugins"),
        temp.path()
            .join("Library")
            .join("Caches")
            .join("JetBrains")
            .join("IntelliJIdea2024.3")
            .join("LocalHistory"),
        temp.path()
            .join("Library")
            .join("Application Support")
            .join("Google")
            .join("AndroidStudio2024.3")
            .join("caches"),
        temp.path()
            .join("AndroidStudioProjects")
            .join("app")
            .join("caches"),
        temp.path()
            .join("Library")
            .join("Android")
            .join("sdk")
            .join("caches"),
        temp.path()
            .join(".android")
            .join("avd")
            .join("Pixel_8.avd")
            .join("cache.img"),
    ] {
        make_non_empty_dir(&path);
    }

    let roots = [
        temp.path().join("Library").join("Application Support"),
        temp.path().join(".config"),
        temp.path().join(".local").join("share"),
        temp.path().join("Library").join("Caches").join("JetBrains"),
        temp.path().join("AndroidStudioProjects"),
        temp.path().join("Library").join("Android"),
        temp.path().join(".android"),
    ];
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.arg("scan");
    for root in roots {
        cmd.arg(root);
    }
    cmd.args(["--json", "--min-size", "0"])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("jetbrains.").not())
        .stdout(predicate::str::contains("android_studio.").not());
}

#[test]
fn home_flag_reports_ide_caches_and_logs_without_app_state() {
    let temp = TempDir::new().unwrap();

    #[cfg(target_os = "macos")]
    let paths = [
        temp.path()
            .join("Library")
            .join("Caches")
            .join("JetBrains")
            .join("IntelliJIdea2024.3")
            .join("caches"),
        temp.path()
            .join("Library")
            .join("Logs")
            .join("JetBrains")
            .join("IntelliJIdea2024.3"),
        temp.path()
            .join("Library")
            .join("Caches")
            .join("Google")
            .join("AndroidStudio2024.3")
            .join("caches"),
        temp.path()
            .join("Library")
            .join("Logs")
            .join("Google")
            .join("AndroidStudio2024.3"),
        temp.path()
            .join("Library")
            .join("Application Support")
            .join("Google")
            .join("AndroidStudio2024.3")
            .join("caches"),
    ];

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let paths = [
        temp.path()
            .join(".cache")
            .join("JetBrains")
            .join("IntelliJIdea2024.3")
            .join("caches"),
        temp.path()
            .join(".cache")
            .join("JetBrains")
            .join("IntelliJIdea2024.3")
            .join("log"),
        temp.path()
            .join(".cache")
            .join("Google")
            .join("AndroidStudio2024.3")
            .join("caches"),
        temp.path()
            .join(".cache")
            .join("Google")
            .join("AndroidStudio2024.3")
            .join("log"),
        temp.path()
            .join(".config")
            .join("Google")
            .join("AndroidStudio2024.3")
            .join("caches"),
    ];

    #[cfg(target_os = "windows")]
    let paths = [
        temp.path()
            .join("AppData")
            .join("Local")
            .join("JetBrains")
            .join("IntelliJIdea2024.3")
            .join("caches"),
        temp.path()
            .join("AppData")
            .join("Local")
            .join("JetBrains")
            .join("IntelliJIdea2024.3")
            .join("log"),
        temp.path()
            .join("AppData")
            .join("Local")
            .join("Google")
            .join("AndroidStudio2024.3")
            .join("caches"),
        temp.path()
            .join("AppData")
            .join("Local")
            .join("Google")
            .join("AndroidStudio2024.3")
            .join("log"),
        temp.path()
            .join("AppData")
            .join("Roaming")
            .join("Google")
            .join("AndroidStudio2024.3")
            .join("caches"),
    ];

    for path in &paths {
        make_non_empty_dir(path);
    }

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    let output = cmd
        .env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains("\"ruleId\": \"jetbrains.system_caches\""));
    assert!(stdout.contains("\"ruleId\": \"jetbrains.logs\""));
    assert!(stdout.contains("\"ruleId\": \"android_studio.system_caches\""));
    assert!(stdout.contains("\"ruleId\": \"android_studio.logs\""));
    assert!(stdout.contains("\"safety\": \"caution\""));
    assert!(!stdout.contains("Application Support/Google/AndroidStudio2024.3/caches"));
    assert!(!stdout.contains(".config/Google/AndroidStudio2024.3/caches"));
    assert!(!stdout.contains("AppData\\\\Roaming\\\\Google\\\\AndroidStudio2024.3\\\\caches"));
}
