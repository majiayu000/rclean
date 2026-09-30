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

#[cfg(all(target_os = "linux", feature = "graveyard"))]
mod cross_fs {
    use super::*;
    use std::os::unix::fs::{MetadataExt, symlink};
    use std::os::unix::net::UnixListener;
    use std::path::Path;

    fn fixtures() -> (TempDir, TempDir) {
        let workspace = TempDir::new().unwrap();
        let graveyard = TempDir::new_in("/dev/shm").unwrap();
        assert_ne!(
            workspace.path().metadata().unwrap().dev(),
            graveyard.path().metadata().unwrap().dev(),
            "fixture must exercise the real EXDEV fallback"
        );
        make_node_project(&workspace);
        (workspace, graveyard)
    }

    fn clean(workspace: &Path, data: &Path) -> Command {
        let mut cmd = Command::cargo_bin("rclean").unwrap();
        cmd.env("XDG_DATA_HOME", data).args([
            "clean",
            workspace.to_str().unwrap(),
            "--all",
            "--graveyard",
            "--yes",
            "--min-size",
            "0",
        ]);
        cmd
    }

    #[test]
    fn graveyard_cross_fs_round_trip_preserves_symlinks_and_manifest() {
        let (workspace, data) = fixtures();
        let candidate = workspace.path().join("node_modules");
        let external = workspace.path().join("external");
        fs::create_dir(&external).unwrap();
        fs::write(external.join("sentinel"), b"external bytes").unwrap();
        symlink(external.join("sentinel"), candidate.join("file-link")).unwrap();
        symlink(&external, candidate.join("dir-link")).unwrap();
        symlink("blob", candidate.join("relative-link")).unwrap();
        symlink("missing", candidate.join("dangling-link")).unwrap();
        let links = ["file-link", "dir-link", "relative-link", "dangling-link"];
        let targets: Vec<_> = links
            .iter()
            .map(|name| fs::read_link(candidate.join(name)).unwrap())
            .collect();

        clean(workspace.path(), data.path()).assert().success();

        let root = data.path().join("rclean/graveyard");
        let manifest = root.join("manifest.jsonl");
        let record: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
        assert_eq!(record["original_path"], candidate.to_str().unwrap());
        assert_eq!(record["rule_id"], "node.node_modules");
        let grave_dir = root.join(record["grave_path"].as_str().unwrap());
        let meta: serde_json::Value =
            serde_json::from_slice(&fs::read(grave_dir.join("meta.json")).unwrap()).unwrap();
        assert_eq!(record, meta, "manifest and per-grave metadata must agree");
        let payload = grave_dir.join("payload");
        for (name, target) in links.iter().zip(&targets) {
            assert!(payload.join(name).symlink_metadata().unwrap().is_symlink());
            assert_eq!(fs::read_link(payload.join(name)).unwrap(), *target);
        }
        assert_eq!(fs::read(payload.join("blob")).unwrap(), b"abc");
        assert!(!candidate.exists());

        Command::cargo_bin("rclean")
            .unwrap()
            .env("XDG_DATA_HOME", data.path())
            .args(["restore", "--id", record["id"].as_str().unwrap()])
            .assert()
            .success();

        for (name, target) in links.iter().zip(&targets) {
            assert!(
                candidate
                    .join(name)
                    .symlink_metadata()
                    .unwrap()
                    .is_symlink()
            );
            assert_eq!(fs::read_link(candidate.join(name)).unwrap(), *target);
        }
        assert_eq!(fs::read(candidate.join("blob")).unwrap(), b"abc");
        assert_eq!(
            fs::read(external.join("sentinel")).unwrap(),
            b"external bytes"
        );
        assert!(!grave_dir.exists());
        assert!(fs::read(&manifest).unwrap().is_empty());
    }

    #[test]
    fn graveyard_cross_fs_copy_failures_keep_source_and_manifest() {
        let (workspace, data) = fixtures();
        let candidate = workspace.path().join("node_modules");
        let socket = candidate.join("socket");
        let listener = UnixListener::bind(&socket).unwrap();

        clean(workspace.path(), data.path()).assert().failure();

        let root = data.path().join("rclean/graveyard");
        let manifest = root.join("manifest.jsonl");
        assert!(!manifest.exists(), "failed bury must not write a record");
        assert_eq!(fs::read(candidate.join("blob")).unwrap(), b"abc");
        assert!(socket.symlink_metadata().is_ok());
        assert!(
            walkdir::WalkDir::new(&root)
                .into_iter()
                .all(|entry| { entry.unwrap().file_name() != "payload" }),
            "failed bury must clean its partial payload"
        );
        drop(listener);
        fs::remove_file(&socket).unwrap();

        clean(workspace.path(), data.path()).assert().success();
        let manifest_before = fs::read(&manifest).unwrap();
        let record: serde_json::Value = serde_json::from_slice(&manifest_before).unwrap();
        let payload = root
            .join(record["grave_path"].as_str().unwrap())
            .join("payload");
        let _listener = UnixListener::bind(payload.join("socket")).unwrap();

        Command::cargo_bin("rclean")
            .unwrap()
            .env("XDG_DATA_HOME", data.path())
            .args(["restore", "--id", record["id"].as_str().unwrap()])
            .assert()
            .failure()
            .stderr(predicate::str::contains("graveyard io"));

        assert!(
            !candidate.exists(),
            "failed restore must clean its partial target"
        );
        assert_eq!(fs::read(payload.join("blob")).unwrap(), b"abc");
        assert!(payload.join("socket").symlink_metadata().is_ok());
        assert_eq!(fs::read(&manifest).unwrap(), manifest_before);
    }
}
