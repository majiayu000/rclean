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

    /// Anchor: on Windows the home directory env var is `USERPROFILE`,
    /// not `HOME`. `output::short_path` currently only reads `HOME`,
    /// so on Windows the table output never collapses to `~`. This
    /// test documents that as the *current* behavior so a future
    /// fix (read `USERPROFILE` on `cfg(windows)`) lands with a
    /// regression-protecting test already in place — flip the
    /// assertion when the fix is in.
    #[test]
    fn userprofile_is_not_yet_collapsed_in_table_output() {
        let temp = TempDir::new().unwrap();
        make_node_project(&temp);

        let mut cmd = Command::cargo_bin("rclean").unwrap();
        cmd.env_remove("HOME")
            .env("USERPROFILE", temp.path().parent().unwrap())
            .args(["scan", temp.path().to_str().unwrap(), "--min-size", "0"])
            .assert()
            .success()
            // Current behavior: USERPROFILE is *not* read, so the
            // absolute path with backslashes (or forward slashes
            // depending on Rust's display impl) is rendered as-is,
            // never as `~/...`. The assertion below documents the
            // gap as a known issue.
            .stdout(predicate::str::contains("~/").not());
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
