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

#[cfg(feature = "graveyard")]
mod graveyard_record_paths {
    use super::*;
    use serde_json::Value;
    use std::path::{Path, PathBuf};

    fn assert_refused(edit: impl FnOnce(&Path, &mut [Value])) {
        let data = TempDir::new().unwrap();
        let first = TempDir::new().unwrap();
        let second = TempDir::new().unwrap();
        for workspace in [&first, &second] {
            make_node_project(workspace);
            Command::cargo_bin("rclean")
                .unwrap()
                .env("XDG_DATA_HOME", data.path())
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
        }
        let root = data.path().join("rclean").join("graveyard");
        let manifest = root.join("manifest.jsonl");
        let mut records: Vec<Value> = fs::read_to_string(&manifest)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let payloads: Vec<_> = records
            .iter()
            .map(|r| {
                root.join(r["grave_path"].as_str().unwrap())
                    .join("payload/blob")
            })
            .collect();
        fs::write(&payloads[1], b"other").unwrap();
        records[0]["expires_at"] = Value::from("2000-01-01T00:00:00Z");
        edit(&root, &mut records);
        let body: String = records
            .iter()
            .map(|r| format!("{}\n", serde_json::to_string(r).unwrap()))
            .collect();
        fs::write(&manifest, &body).unwrap();
        let target = first.path().join("missing/restored");
        for args in [
            vec!["graveyard", "gc", "--dry-run"],
            vec!["graveyard", "gc"],
            vec![
                "restore",
                "--id",
                records[0]["id"].as_str().unwrap(),
                "--to",
                target.to_str().unwrap(),
            ],
        ] {
            Command::cargo_bin("rclean")
                .unwrap()
                .env("XDG_DATA_HOME", data.path())
                .args(&args)
                .assert()
                .code(1)
                .stderr(predicate::str::contains("grave_path"));
            assert!(!target.parent().unwrap().exists());
            assert_eq!(fs::read(&payloads[0]).unwrap(), b"abc");
            assert_eq!(fs::read(&payloads[1]).unwrap(), b"other");
            assert_eq!(fs::read(&manifest).unwrap(), body.as_bytes());
        }
    }

    #[test]
    fn shared_parent_is_refused() {
        assert_refused(|_, records| {
            let path = PathBuf::from(records[0]["grave_path"].as_str().unwrap());
            let year = path
                .components()
                .next()
                .unwrap()
                .as_os_str()
                .to_str()
                .unwrap();
            assert!(Path::new(records[1]["grave_path"].as_str().unwrap()).starts_with(year));
            records[0]["grave_path"] = Value::from(year);
        });
    }

    #[test]
    fn another_leaf_is_refused() {
        assert_refused(|_, records| records[0]["grave_path"] = records[1]["grave_path"].clone());
    }

    #[cfg(unix)]
    #[test]
    fn in_root_symlink_is_refused() {
        assert_refused(|root, records| {
            std::os::unix::fs::symlink(
                root.join(records[1]["grave_path"].as_str().unwrap()),
                root.join("alias"),
            )
            .unwrap();
            records[0]["grave_path"] = Value::from("alias");
        });
    }

    fn replace_date_parent(root: &Path, records: &[Value], link: impl FnOnce(&Path, &Path)) {
        let path = Path::new(records[0]["grave_path"].as_str().unwrap());
        let year = root.join(path.components().next().unwrap());
        let saved = root.join("saved");
        fs::rename(&year, &saved).unwrap();
        link(&saved, &year);
    }

    #[cfg(unix)]
    #[test]
    fn own_path_with_symlink_parent_is_refused() {
        assert_refused(|root, records| {
            replace_date_parent(root, records, |saved, year| {
                std::os::unix::fs::symlink(saved, year).unwrap()
            })
        });
    }

    #[cfg(windows)]
    #[test]
    fn own_path_with_symlink_parent_is_refused() {
        assert_refused(|root, records| {
            replace_date_parent(root, records, |saved, year| {
                std::os::windows::fs::symlink_dir(saved, year).unwrap()
            })
        });
    }

    #[cfg(windows)]
    #[test]
    fn own_path_with_junction_parent_is_refused() {
        assert_refused(|root, records| {
            replace_date_parent(root, records, |saved, year| {
                let output = std::process::Command::new("cmd")
                    .args(["/C", "mklink", "/J"])
                    .arg(year)
                    .arg(saved)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "mklink failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            })
        });
    }
}
