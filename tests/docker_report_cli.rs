use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn docker_report_missing_binary_is_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let missing = temp.path().join("missing-docker");

    let output = Command::cargo_bin("rclean")?
        .env("RCLEAN_DOCKER_BIN", &missing)
        .args(["docker", "report", "--json"])
        .assert()
        .code(3)
        .get_output()
        .stdout
        .clone();
    let report: Value = serde_json::from_slice(&output)?;

    assert_eq!(report["status"]["kind"].as_str(), Some("unavailable"));
    assert!(
        report["status"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("Docker CLI not found"))
    );
    assert_eq!(report["summary"]["resources"].as_u64(), Some(0));
    Ok(())
}

#[cfg(unix)]
#[test]
fn docker_report_permission_denied_is_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let fake = write_fake_docker(
        temp.path(),
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$RCLEAN_FAKE_DOCKER_LOG"
printf 'permission denied while trying to connect to the Docker daemon socket\n' >&2
exit 1
"#,
    )?;

    let output = Command::cargo_bin("rclean")?
        .env("RCLEAN_DOCKER_BIN", &fake)
        .env("RCLEAN_FAKE_DOCKER_LOG", temp.path().join("docker.log"))
        .args(["docker", "report", "--json", "--timeout", "30s"])
        .assert()
        .code(3)
        .get_output()
        .stdout
        .clone();
    let report: Value = serde_json::from_slice(&output)?;

    assert_eq!(report["status"]["kind"].as_str(), Some("permissionDenied"));
    assert!(
        report["status"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("permission denied"))
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn docker_report_timeout_is_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let fake = write_fake_docker(
        temp.path(),
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$RCLEAN_FAKE_DOCKER_LOG"
sleep 5
"#,
    )?;

    let output = Command::cargo_bin("rclean")?
        .env("RCLEAN_DOCKER_BIN", &fake)
        .env("RCLEAN_FAKE_DOCKER_LOG", temp.path().join("docker.log"))
        .args(["docker", "report", "--json", "--timeout", "1s"])
        .assert()
        .code(3)
        .get_output()
        .stdout
        .clone();
    let report: Value = serde_json::from_slice(&output)?;

    assert_eq!(report["status"]["kind"].as_str(), Some("timedOut"));
    assert_eq!(report["status"]["timeoutMs"].as_u64(), Some(1000));
    Ok(())
}

/// Regression for #350: a failed probe must not borrow the sentence
/// used for a genuinely empty result. rclean did not manage to look,
/// so it cannot report that there is nothing to reclaim.
#[cfg(unix)]
#[test]
fn docker_report_human_failure_never_claims_an_empty_result()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let fake = write_fake_docker(
        temp.path(),
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$RCLEAN_FAKE_DOCKER_LOG"
sleep 5
"#,
    )?;

    let output = Command::cargo_bin("rclean")?
        .env("RCLEAN_DOCKER_BIN", &fake)
        .env("RCLEAN_FAKE_DOCKER_LOG", temp.path().join("docker.log"))
        .args(["docker", "report", "--timeout", "1s"])
        .assert()
        .code(3)
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output)?;

    assert!(
        !stdout.contains("No Docker cleanup resources reported."),
        "a failed probe must not assert an empty result, got: {stdout}"
    );
    assert!(
        stdout.contains("Docker was not queried successfully"),
        "a failed probe must say the query failed, got: {stdout}"
    );
    assert!(
        stdout.contains("--timeout"),
        "the failure should point at the flag that fixes it, got: {stdout}"
    );
    Ok(())
}

/// The counterpart: a probe that succeeds and finds nothing
/// reclaimable renders its (zero-count) table and must never carry the
/// failure wording, so the two cases stay distinguishable.
#[cfg(unix)]
#[test]
fn docker_report_human_success_with_nothing_to_clean_is_not_a_failure()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    // Reports a live daemon, then an empty `system df`.
    let fake = write_fake_docker(
        temp.path(),
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$RCLEAN_FAKE_DOCKER_LOG"
case "$*" in
  *"version"*) printf '29.5.3\n' ;;
  *) : ;;
esac
"#,
    )?;

    let output = Command::cargo_bin("rclean")?
        .env("RCLEAN_DOCKER_BIN", &fake)
        .env("RCLEAN_FAKE_DOCKER_LOG", temp.path().join("docker.log"))
        .args(["docker", "report", "--timeout", "30s"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output)?;

    assert!(
        stdout.contains("Docker: available"),
        "a successful probe must report the daemon as available, got: {stdout}"
    );
    assert!(
        !stdout.contains("Docker was not queried successfully"),
        "a successful query must not claim it failed, got: {stdout}"
    );
    // Zero-count rows still render, which is how a real successful
    // probe with nothing reclaimable looks: the taxonomy is always
    // emitted rather than collapsing into a single sentence.
    assert!(
        stdout.contains("docker.dangling_images"),
        "the resource taxonomy should still render, got: {stdout}"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn docker_report_success_is_report_only_and_never_prunes() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = TempDir::new()?;
    let log = temp.path().join("docker.log");
    let fake = write_successful_fake_docker(temp.path())?;

    let output = Command::cargo_bin("rclean")?
        .env("RCLEAN_DOCKER_BIN", &fake)
        .env("RCLEAN_FAKE_DOCKER_LOG", &log)
        .args(["docker", "report", "--json", "--timeout", "30s"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: Value = serde_json::from_slice(&output)?;
    let resources = report["resources"]
        .as_array()
        .expect("resources should be an array");
    let ids: Vec<&str> = resources
        .iter()
        .filter_map(|resource| resource["resourceId"].as_str())
        .collect();

    assert_eq!(report["status"]["kind"].as_str(), Some("available"));
    assert_eq!(report["summary"]["selectedResources"].as_u64(), Some(0));
    assert!(ids.contains(&"docker.build_cache"));
    assert!(ids.contains(&"docker.images"));
    assert!(ids.contains(&"docker.local_volumes"));
    assert!(ids.contains(&"docker.dangling_images"));
    assert!(ids.contains(&"docker.stopped_containers"));
    assert!(ids.contains(&"docker.networks"));
    assert!(
        resources
            .iter()
            .all(|resource| resource["selected"].as_bool() == Some(false))
    );

    let commands = std::fs::read_to_string(&log)?;
    assert!(commands.contains("version --format {{json .Server}}"));
    assert!(commands.contains("system df --format {{json .}}"));
    assert!(!commands.contains("prune"));
    assert!(!commands.contains(" rm"));
    assert!(!commands.contains("delete"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn docker_report_large_output_is_explicit_error() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let fake = write_fake_docker(
        temp.path(),
        r#"#!/bin/sh
case "$*" in
  'version --format {{json .Server}}')
    printf '{"Version":"27.0.0"}\n'
    ;;
  'system df --format {{json .}}')
    i=0
    while [ "$i" -lt 5000 ]; do
      printf '{"Type":"Images","TotalCount":"2","Active":"1","Size":"1GB","Reclaimable":"500MB"}\n'
      i=$((i + 1))
    done
    ;;
  *)
    printf '{"ID":"ok"}\n'
    ;;
esac
"#,
    )?;

    let output = Command::cargo_bin("rclean")?
        .env("RCLEAN_DOCKER_BIN", &fake)
        .args(["docker", "report", "--json", "--timeout", "30s"])
        .assert()
        .code(3)
        .get_output()
        .stdout
        .clone();
    let report: Value = serde_json::from_slice(&output)?;

    assert_eq!(report["status"]["kind"].as_str(), Some("error"));
    assert!(
        report["status"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("exceeded 65536 bytes")),
        "unexpected report: {report:#?}"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn doctor_default_does_not_probe_docker() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let log = temp.path().join("docker.log");
    let fake = write_successful_fake_docker(temp.path())?;

    Command::cargo_bin("rclean")?
        .env("HOME", temp.path())
        .env("RCLEAN_DOCKER_BIN", &fake)
        .env("RCLEAN_FAKE_DOCKER_LOG", &log)
        .arg("doctor")
        .assert()
        .code(3)
        .stdout(predicate::str::contains("docker.daemon").not());

    assert!(
        !log.exists(),
        "plain doctor must remain pure path checks and not invoke Docker"
    );
    Ok(())
}

#[test]
fn doctor_docker_probe_reports_missing_binary() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let missing = temp.path().join("missing-docker");

    Command::cargo_bin("rclean")?
        .env("HOME", temp.path())
        .env("RCLEAN_DOCKER_BIN", &missing)
        .args(["doctor", "--docker"])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("docker.daemon"))
        .stdout(predicate::str::contains("Docker CLI not found"));
    Ok(())
}

#[cfg(unix)]
fn write_successful_fake_docker(dir: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    write_fake_docker(
        dir,
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$RCLEAN_FAKE_DOCKER_LOG"
case "$*" in
  'version --format {{json .Server}}')
    printf '{"Version":"27.0.0"}\n'
    ;;
  'system df --format {{json .}}')
    printf '{"Type":"Images","TotalCount":"2","Active":"1","Size":"1GB","Reclaimable":"500MB (50%%)"}\n'
    printf '{"Type":"Build Cache","TotalCount":"3","Active":"0","Size":"2GB","Reclaimable":"2GB"}\n'
    printf '{"Type":"Local Volumes","TotalCount":"4","Active":"2","Size":"4GB","Reclaimable":"1GB (25%%)"}\n'
    ;;
  'image ls --filter dangling=true --format {{json .}}')
    printf '{"ID":"sha256:deadbeef"}\n'
    ;;
  'container ls --all --filter status=exited --format {{json .}}')
    printf '{"ID":"abc123","Names":"named-container"}\n'
    ;;
  'network ls --format {{json .}}')
    printf '{"ID":"net1","Name":"bridge"}\n'
    ;;
  *)
    printf 'unexpected docker command: %s\n' "$*" >&2
    exit 40
    ;;
esac
"#,
    )
}

#[cfg(unix)]
fn write_fake_docker(dir: &std::path::Path, script: &str) -> std::io::Result<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join("docker");
    std::fs::write(&path, script)?;
    let mut permissions = std::fs::metadata(&path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions)?;
    Ok(path)
}
