use assert_cmd::Command;
use predicates::prelude::*;
#[cfg(target_os = "macos")]
use serde_json::Value;
#[cfg(target_os = "macos")]
use std::path::Path;
#[cfg(unix)]
use tempfile::TempDir;

#[cfg(target_os = "macos")]
fn make_non_empty_dir(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(path)?;
    std::fs::write(path.join("blob"), b"x")?;
    Ok(())
}

#[test]
fn system_flag_conflicts_with_positional_paths_home_and_tmp() {
    let mut with_path = Command::cargo_bin("rclean").unwrap();
    with_path
        .args(["scan", "--system", "/tmp/somepath"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("cannot be used with")
                .or(predicate::str::contains("conflicts with")),
        );

    let mut with_home = Command::cargo_bin("rclean").unwrap();
    with_home
        .args(["scan", "--system", "--home"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("cannot be used with")
                .or(predicate::str::contains("conflicts with")),
        );

    let mut with_tmp = Command::cargo_bin("rclean").unwrap();
    with_tmp
        .args(["scan", "--system", "--tmp"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("cannot be used with")
                .or(predicate::str::contains("conflicts with")),
        );
}

#[cfg(target_os = "macos")]
#[test]
fn system_flag_reports_only_idleassetsd_anchor() -> Result<(), Box<dyn std::error::Error>> {
    let system = TempDir::new()?;
    let home = TempDir::new()?;
    let app_support = system.path().join("Library").join("Application Support");
    let idleassetsd = app_support.join("com.apple.idleassetsd");
    let sibling = app_support.join("node_modules");
    make_non_empty_dir(&idleassetsd)?;
    make_non_empty_dir(&sibling)?;

    let mut cmd = Command::cargo_bin("rclean")?;
    let output = cmd
        .env("HOME", home.path())
        .env("RCLEAN_TEST_SYSTEM_ROOTS", &idleassetsd)
        .args(["scan", "--system", "--json", "--min-size", "0"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: Value = serde_json::from_slice(&output)?;
    let canonical_idleassetsd = idleassetsd.canonicalize()?.display().to_string();
    let candidates: Vec<&Value> = report["projects"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|project| project["candidates"].as_array().into_iter().flatten())
        .collect();

    assert_eq!(
        report["roots"][0].as_str(),
        Some(canonical_idleassetsd.as_str())
    );
    assert_eq!(
        candidates.len(),
        1,
        "unexpected candidates: {candidates:#?}"
    );
    assert_eq!(
        candidates[0]["path"].as_str(),
        Some(canonical_idleassetsd.as_str())
    );
    assert_eq!(candidates[0]["ruleId"].as_str(), Some("apple.idleassetsd"));
    assert_eq!(candidates[0]["safety"].as_str(), Some("report-only"));
    assert_eq!(candidates[0]["requiresSudo"].as_bool(), Some(true));
    assert!(
        !serde_json::to_string(&report)?.contains(&sibling.display().to_string()),
        "system scan must not traverse sibling roots"
    );
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn system_flag_rejects_user_level_idleassetsd_anchor() -> Result<(), Box<dyn std::error::Error>> {
    let home = TempDir::new()?;
    let user_idleassetsd = home
        .path()
        .join("Library")
        .join("Application Support")
        .join("com.apple.idleassetsd");
    make_non_empty_dir(&user_idleassetsd)?;

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", home.path())
        .env("RCLEAN_TEST_SYSTEM_ROOTS", &user_idleassetsd)
        .args(["scan", "--system", "--json", "--min-size", "0"])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("\"candidates\": 0"))
        .stdout(predicate::str::contains("apple.idleassetsd").not());
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn clean_system_all_reports_manual_sudo_reason_but_selects_none()
-> Result<(), Box<dyn std::error::Error>> {
    let system = TempDir::new()?;
    let home = TempDir::new()?;
    let idleassetsd = system
        .path()
        .join("Library")
        .join("Application Support")
        .join("com.apple.idleassetsd");
    make_non_empty_dir(&idleassetsd)?;

    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", home.path())
        .env("RCLEAN_TEST_SYSTEM_ROOTS", &idleassetsd)
        .args(["clean", "--system", "--all", "--dry-run", "--min-size", "0"])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("report-only"))
        .stdout(predicate::str::contains("will not run sudo"))
        .stdout(predicate::str::contains("Nothing selected."));

    assert!(idleassetsd.exists(), "report-only clean must not delete");
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn system_scan_plan_retains_requires_sudo_in_project_candidate()
-> Result<(), Box<dyn std::error::Error>> {
    let system = TempDir::new()?;
    let home = TempDir::new()?;
    let plan = system.path().join("plan.json");
    let idleassetsd = system
        .path()
        .join("Library")
        .join("Application Support")
        .join("com.apple.idleassetsd");
    make_non_empty_dir(&idleassetsd)?;

    let mut scan = Command::cargo_bin("rclean")?;
    scan.env("HOME", home.path())
        .env("RCLEAN_TEST_SYSTEM_ROOTS", &idleassetsd)
        .args([
            "scan",
            "--system",
            "--write-plan",
            plan.to_str().unwrap(),
            "--min-size",
            "0",
        ])
        .assert()
        .success();

    let plan_json: Value = serde_json::from_str(&std::fs::read_to_string(&plan)?)?;
    assert_eq!(plan_json["selected"].as_array().unwrap().len(), 0);
    let candidate = &plan_json["projects"][0]["candidates"][0];
    assert_eq!(candidate["ruleId"].as_str(), Some("apple.idleassetsd"));
    assert_eq!(candidate["requiresSudo"].as_bool(), Some(true));
    Ok(())
}

#[cfg(unix)]
#[test]
fn clean_plan_requires_sudo_refuses_without_invoking_sudo() -> Result<(), Box<dyn std::error::Error>>
{
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new()?;
    let bin = temp.path().join("bin");
    let fake_sudo = bin.join("sudo");
    let sudo_log = temp.path().join("sudo-called");
    std::fs::create_dir(&bin)?;
    std::fs::write(
        &fake_sudo,
        "#!/bin/sh\nprintf 'sudo called\\n' > \"$RCLEAN_FAKE_SUDO_LOG\"\nexit 0\n",
    )?;
    let mut permissions = std::fs::metadata(&fake_sudo)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_sudo, permissions)?;

    let plan = temp.path().join("plan.json");
    let candidate_path = temp
        .path()
        .join("Library")
        .join("Application Support")
        .join("com.apple.idleassetsd");
    let plan_json = serde_json::json!({
        "schemaVersion": 2,
        "toolVersion": "0.1.0",
        "generatedAt": "2026-05-06T00:00:00Z",
        "deleteMode": "trash",
        "roots": [temp.path().display().to_string()],
        "summary": {
            "projectsScanned": 1,
            "projectsWithCandidates": 1,
            "candidates": 1,
            "safeCandidates": 0,
            "cautionCandidates": 0,
            "blockedCandidates": 0,
            "reportOnlyCandidates": 1,
            "totalBytes": 0
        },
        "selected": [{
            "id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            "path": candidate_path.display().to_string(),
            "ruleId": "apple.idleassetsd",
            "bytes": 0,
            "safety": "report-only",
            "requiresSudo": true,
            "category": "cache",
            "riskScore": 0.0
        }],
        "projects": []
    });
    std::fs::write(&plan, serde_json::to_string_pretty(&plan_json)?)?;

    let old_path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(&old_path));
    let path = std::env::join_paths(paths)?;

    let mut clean = Command::cargo_bin("rclean")?;
    clean
        .env("PATH", path)
        .env("RCLEAN_FAKE_SUDO_LOG", &sudo_log)
        .args(["clean", "--plan", plan.to_str().unwrap(), "--yes"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("will not run sudo"));

    assert!(!sudo_log.exists(), "rclean must not invoke sudo");
    Ok(())
}
