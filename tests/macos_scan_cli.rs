#![cfg(target_os = "macos")]

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn home_flag_reports_macos_high_value_candidates() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    for path in [
        temp.path()
            .join("Library")
            .join("Caches")
            .join("LarkInternational"),
        temp.path()
            .join("Library")
            .join("Application Support")
            .join("Google")
            .join("Chrome")
            .join("OptGuideOnDeviceModel"),
        temp.path()
            .join("Library")
            .join("Application Support")
            .join("Google")
            .join("Chrome")
            .join("Default"),
        temp.path()
            .join("Library")
            .join("Application Support")
            .join("LarkInternational")
            .join("update"),
        temp.path()
            .join("Library")
            .join("Application Support")
            .join("com.apple.wallpaper")
            .join("aerials")
            .join("videos"),
    ] {
        std::fs::create_dir_all(&path)?;
        std::fs::write(path.join("blob"), "x")?;
    }

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", temp.path())
        .args(["scan", "--home", "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ruleId\": \"app.lark_cache\""))
        .stdout(predicate::str::contains(
            "\"ruleId\": \"chrome.opt_guide_model\"",
        ))
        .stdout(predicate::str::contains("\"ruleId\": \"app.lark_update\""))
        .stdout(predicate::str::contains(
            "\"ruleId\": \"apple.wallpaper_aerial_videos\"",
        ))
        .stdout(predicate::str::contains("/Default\"").not())
        .stdout(predicate::str::contains("\"safety\": \"caution\""));
    Ok(())
}
