//! Anchor file for platform-specific behavior tests.
//!
//! Each `#[cfg(unix)]` / `#[cfg(windows)]` block exercises a code
//! path that depends on the OS. CI's Windows runner skips
//! unix-gated tests and vice-versa, so a regression on either
//! platform fails CI loudly instead of silently mismatching.
//!
//! New cross-platform behaviors should land here rather than in
//! `tests/cli.rs` so the platform-vs-portable split stays obvious
//! when reading the test suite.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Builds a minimal Node project: `package.json` + a populated
/// `node_modules` so scan emits exactly one candidate.
fn make_node_project(temp: &TempDir) {
    fs::write(temp.path().join("package.json"), "{}").unwrap();
    fs::create_dir(temp.path().join("node_modules")).unwrap();
    fs::write(temp.path().join("node_modules").join("blob"), b"abc").unwrap();
}

#[cfg(feature = "graveyard")]
#[test]
fn graveyard_gc_delete_failure_keeps_payload_and_manifest() {
    let workspace = TempDir::new().unwrap();
    let graveyard = TempDir::new().unwrap();
    make_node_project(&workspace);
    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args([
            "clean",
            workspace.path().to_str().unwrap(),
            "--all",
            "--graveyard",
            "--yes",
            "--min-size",
            "0",
        ])
        .assert()
        .success();

    let root = graveyard.path().join("rclean/graveyard");
    let manifest = root.join("manifest.jsonl");
    let mut record: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    record["expires_at"] = serde_json::json!("2000-01-01T00:00:00Z");
    let before = format!("{}\n", serde_json::to_string(&record).unwrap());
    fs::write(&manifest, &before).unwrap();
    let payload = root
        .join(record["grave_path"].as_str().unwrap())
        .join("payload");

    #[cfg(unix)]
    let permissions = {
        use std::os::unix::fs::PermissionsExt;
        let original = fs::metadata(&payload).unwrap().permissions();
        fs::set_permissions(&payload, fs::Permissions::from_mode(0o500)).unwrap();
        let probe_path = payload.join("permission_probe");
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe_path)
        {
            Ok(probe) => {
                drop(probe);
                fs::remove_file(probe_path).unwrap();
                fs::set_permissions(&payload, original).unwrap();
                eprintln!("skipping: current privileges bypass directory write permissions");
                return;
            }
            Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied),
        }
        original
    };
    #[cfg(windows)]
    let locked_file = {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(payload.join("blob"))
            .unwrap()
    };

    let preview = Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["graveyard", "gc", "--dry-run"])
        .output()
        .unwrap();
    let output = Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["graveyard", "gc"])
        .output()
        .unwrap();

    #[cfg(unix)]
    fs::set_permissions(&payload, permissions).unwrap();
    #[cfg(windows)]
    drop(locked_file);

    assert!(preview.status.success());
    assert!(
        String::from_utf8(preview.stderr)
            .unwrap()
            .contains("would remove 1")
    );
    assert!(!output.status.success(), "gc must fail when deletion fails");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("graveyard io"), "{stderr}");
    assert!(!stderr.contains("removed 1 expired grave(s)"), "{stderr}");
    assert_eq!(fs::read(payload.join("blob")).unwrap(), b"abc");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(&manifest).unwrap()).unwrap(),
        record
    );
    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["graveyard", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(record["id"].as_str().unwrap()));

    Command::cargo_bin("rclean")
        .unwrap()
        .env("XDG_DATA_HOME", graveyard.path())
        .args(["graveyard", "gc"])
        .assert()
        .success()
        .stderr(predicate::str::contains("removed 1 expired grave(s)"));
    assert!(!payload.exists());
    assert!(fs::read(&manifest).unwrap().is_empty());
}

/// Smoke test for all platforms: an empty workspace returns a
/// valid JSON document (no candidates), not a panic or malformed
/// output. Guards against output-formatter regressions.
#[test]
fn scan_empty_workspace_emits_valid_json_on_all_platforms() {
    let temp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["scan", temp.path().to_str().unwrap(), "--json"])
        .assert()
        // Exit 3 = scan succeeded but found 0 candidates. Platform-
        // independent.
        .code(3)
        .stdout(predicate::str::contains("\"projects\": []"))
        .stdout(predicate::str::contains("\"candidates\": 0"));
}

#[cfg(unix)]
mod unix {
    use super::*;

    /// `output::short_path` collapses `$HOME` to `~` in scan-table
    /// output. The function reads `HOME`, which is well-defined on
    /// Unix. CI's macos-latest + ubuntu-latest runners both set it.
    #[test]
    fn home_prefix_is_collapsed_to_tilde_in_table_output() {
        let temp = TempDir::new().unwrap();
        make_node_project(&temp);
        // scan() canonicalizes its roots, so on macOS the path
        // rclean compares against is `/private/var/...` rather than
        // `/var/...`. Use the canonical form for HOME so the prefix
        // actually matches in `output::short_path`.
        let canonical = temp.path().canonicalize().unwrap();
        let parent = canonical.parent().unwrap();
        let leaf = canonical.file_name().unwrap().to_str().unwrap();

        let mut cmd = Command::cargo_bin("rclean").unwrap();
        cmd.env("HOME", parent)
            .args(["scan", temp.path().to_str().unwrap(), "--min-size", "0"])
            .assert()
            .success()
            // With HOME set to the parent of the tempdir, the project
            // path in the table should render as `~/<tempdir-name>`,
            // not the absolute path.
            .stdout(predicate::str::contains(format!("~/{leaf}")));
    }

    /// A symlink candidate is classified as blocked on Unix. Mirrors
    /// the scan.rs unit test but at the binary boundary; protects
    /// against output-layer regressions that would hide the safety
    /// downgrade from `--json` consumers.
    #[test]
    fn symlink_to_node_modules_is_blocked_on_unix() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("package.json"), "{}").unwrap();
        let real = temp.path().join("real_modules");
        fs::create_dir(&real).unwrap();
        let link = temp.path().join("node_modules");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let mut cmd = Command::cargo_bin("rclean").unwrap();
        cmd.args([
            "scan",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
            "--include-blocked",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"safety\": \"blocked\""));
    }
}

#[cfg(windows)]
mod windows {
    use super::*;

    /// Windows uses `USERPROFILE` as its home directory env var.
    /// `scan()` canonicalizes roots into extended paths, so the test
    /// uses the canonical parent as `USERPROFILE` to assert the exact
    /// prefix-collapsing behavior the CLI sees on CI.
    #[test]
    fn userprofile_prefix_is_collapsed_to_tilde_in_table_output() {
        let temp = TempDir::new().unwrap();
        make_node_project(&temp);
        let canonical = temp.path().canonicalize().unwrap();
        let parent = canonical.parent().unwrap();
        let leaf = canonical.file_name().unwrap().to_str().unwrap();

        let mut cmd = Command::cargo_bin("rclean").unwrap();
        cmd.env_remove("HOME")
            .env("USERPROFILE", parent)
            .args(["scan", temp.path().to_str().unwrap(), "--min-size", "0"])
            .assert()
            .success()
            .stdout(predicate::str::contains(format!("~\\{leaf}")));
    }

    /// A symlink_dir candidate is classified as blocked on Windows.
    /// Note: creating a directory symlink on Windows requires either
    /// admin or Developer Mode. CI runners (windows-latest) have
    /// Developer Mode enabled, so this should pass in CI even if
    /// it fails on a locked-down local dev box.
    #[test]
    fn symlink_to_node_modules_is_blocked_on_windows() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("package.json"), "{}").unwrap();
        let real = temp.path().join("real_modules");
        fs::create_dir(&real).unwrap();
        let link = temp.path().join("node_modules");
        // Returns an error on non-Developer-Mode Windows; skip the
        // test rather than fail.
        if std::os::windows::fs::symlink_dir(&real, &link).is_err() {
            eprintln!("skipping: directory symlink creation needs admin or Developer Mode");
            return;
        }

        let mut cmd = Command::cargo_bin("rclean").unwrap();
        cmd.args([
            "scan",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
            "--include-blocked",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"safety\": \"blocked\""));
    }
}
