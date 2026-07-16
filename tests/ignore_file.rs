use assert_cmd::Command;
use predicates::prelude::*;
#[cfg(unix)]
use serde_json::Value;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::PathBuf;
use tempfile::TempDir;

#[cfg(unix)]
struct RestorePermissions {
    path: PathBuf,
    mode: u32,
}

#[cfg(unix)]
impl Drop for RestorePermissions {
    fn drop(&mut self) {
        let Ok(metadata) = std::fs::metadata(&self.path) else {
            eprintln!(
                "could not restore permissions for missing {}",
                self.path.display()
            );
            return;
        };
        let mut permissions = metadata.permissions();
        permissions.set_mode(self.mode);
        if let Err(error) = std::fs::set_permissions(&self.path, permissions) {
            eprintln!(
                "could not restore permissions for {}: {error}",
                self.path.display()
            );
        }
    }
}

fn build_node_project(temp: &TempDir) {
    std::fs::write(temp.path().join("package.json"), "{}").unwrap();
    std::fs::create_dir(temp.path().join("node_modules")).unwrap();
    std::fs::write(temp.path().join("node_modules").join("blob"), "abc").unwrap();
}

#[test]
fn baseline_without_ignore_emits_node_modules() {
    // Sanity: without any .rcleanignore or --ignore, the candidate appears.
    let temp = TempDir::new().unwrap();
    build_node_project(&temp);

    Command::cargo_bin("rclean")
        .unwrap()
        .args([
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
        ));
}

#[test]
fn rcleanignore_file_excludes_candidate() {
    let temp = TempDir::new().unwrap();
    build_node_project(&temp);
    std::fs::write(temp.path().join(".rcleanignore"), "node_modules/\n").unwrap();

    // Exit code 3 = scan found 0 candidates. That's the success signal here:
    // the ignore matcher dropped node_modules before classification.
    Command::cargo_bin("rclean")
        .unwrap()
        .args([
            "scan",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
        ])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("\"ruleId\":").not())
        .stdout(predicate::str::contains("\"candidates\": 0"));
}

#[test]
fn cli_ignore_flag_excludes_candidate() {
    let temp = TempDir::new().unwrap();
    build_node_project(&temp);

    Command::cargo_bin("rclean")
        .unwrap()
        .args([
            "scan",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
            "--ignore",
            "**/node_modules/",
        ])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("\"ruleId\":").not())
        .stdout(predicate::str::contains("\"candidates\": 0"));
}

#[test]
fn rcleanignore_negation_re_includes_candidate() {
    // Pattern excludes everything, then re-includes node_modules — verifies the
    // gitignore matcher's negation (`!` prefix) is honored end-to-end.
    let temp = TempDir::new().unwrap();
    build_node_project(&temp);
    std::fs::write(
        temp.path().join(".rcleanignore"),
        "*\n!node_modules/\n!package.json\n",
    )
    .unwrap();

    Command::cargo_bin("rclean")
        .unwrap()
        .args([
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
        ));
}

#[test]
fn rcleanignore_at_root_excludes_deeply_nested_candidate() {
    // Review on #35 flagged that the existing tests only exercise
    // root-level candidates. .gitignore semantics treat `node_modules/`
    // as a path-anywhere pattern (not anchored), so a root-level
    // `.rcleanignore` should also drop candidates two levels deep.
    // If `ignore` crate's strip_prefix logic ever breaks, this test
    // fails loudly.
    let temp = TempDir::new().unwrap();
    let nested = temp.path().join("apps").join("web");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("package.json"), "{}").unwrap();
    std::fs::create_dir(nested.join("node_modules")).unwrap();
    std::fs::write(nested.join("node_modules").join("blob"), "abc").unwrap();

    std::fs::write(temp.path().join(".rcleanignore"), "node_modules/\n").unwrap();

    Command::cargo_bin("rclean")
        .unwrap()
        .args([
            "scan",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
        ])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("\"ruleId\":").not());
}

#[test]
fn multiple_ignore_flags_layer_on_top_of_each_other() {
    // Two separate Node projects, each excluded by its own --ignore glob.
    // Verifies clap's Vec<String> collects both globs and the matcher
    // applies them additively (not just the last one winning).
    let temp = TempDir::new().unwrap();
    for name in ["app1", "app2"] {
        let app = temp.path().join(name);
        std::fs::create_dir(&app).unwrap();
        std::fs::write(app.join("package.json"), "{}").unwrap();
        std::fs::create_dir(app.join("node_modules")).unwrap();
        std::fs::write(app.join("node_modules").join("blob"), "abc").unwrap();
    }

    Command::cargo_bin("rclean")
        .unwrap()
        .args([
            "scan",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
            "--ignore",
            "**/app1/**",
            "--ignore",
            "**/app2/**",
        ])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("\"ruleId\":").not());
}

#[test]
fn first_ignore_glob_miss_does_not_block_second_glob_hit() {
    // Edge case: --ignore A doesn't match anything, --ignore B does.
    // The "last glob wins" anti-pattern would let the candidate through;
    // additive layering keeps it excluded.
    let temp = TempDir::new().unwrap();
    build_node_project(&temp);

    Command::cargo_bin("rclean")
        .unwrap()
        .args([
            "scan",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
            "--ignore",
            "**/nonexistent_dir/",
            "--ignore",
            "**/node_modules/",
        ])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("\"ruleId\":").not());
}

#[test]
fn invalid_rcleanignore_is_reported_as_scan_warning() {
    let temp = TempDir::new().unwrap();
    build_node_project(&temp);
    std::fs::write(temp.path().join(".rcleanignore"), "{a,b\n").unwrap();

    Command::cargo_bin("rclean")
        .unwrap()
        .args([
            "scan",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"warnings\": ["))
        .stdout(predicate::str::contains("\"kind\": \"ignoreFileLoad\""))
        .stdout(predicate::str::contains(".rcleanignore"))
        .stdout(predicate::str::contains(
            "\"ruleId\": \"node.node_modules\"",
        ));
}

#[test]
fn invalid_rcleanignore_table_reports_warning_summary() {
    let temp = TempDir::new().unwrap();
    build_node_project(&temp);
    std::fs::write(temp.path().join(".rcleanignore"), "{a,b\n").unwrap();

    Command::cargo_bin("rclean")
        .unwrap()
        .args(["scan", temp.path().to_str().unwrap(), "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Warnings during scan:"))
        .stdout(predicate::str::contains("Results may be incomplete."));
}

#[test]
fn invalid_cli_ignore_glob_fails_scan() {
    let temp = TempDir::new().unwrap();
    build_node_project(&temp);

    Command::cargo_bin("rclean")
        .unwrap()
        .args([
            "scan",
            temp.path().to_str().unwrap(),
            "--json",
            "--min-size",
            "0",
            "--ignore",
            "{a,b",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid --ignore glob"));
}

#[cfg(unix)]
#[test]
fn candidate_size_scan_warning_preserves_partial_bytes_and_output_contract() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("Cargo.toml"), "").unwrap();
    let target = temp.path().join("target");
    let readable = target.join("readable");
    let denied = target.join("denied");
    std::fs::create_dir_all(&readable).unwrap();
    std::fs::create_dir(&denied).unwrap();
    std::fs::write(readable.join("kept.bin"), [0; 4096]).unwrap();

    let original_mode = std::fs::metadata(&denied).unwrap().permissions().mode();
    let _restore = RestorePermissions {
        path: denied.clone(),
        mode: original_mode,
    };
    let mut permissions = std::fs::metadata(&denied).unwrap().permissions();
    permissions.set_mode(0o000);
    std::fs::set_permissions(&denied, permissions).unwrap();

    let run_json = |min_size: &str| {
        Command::cargo_bin("rclean")
            .unwrap()
            .args([
                "scan",
                temp.path().to_str().unwrap(),
                "--json",
                "--min-size",
                min_size,
            ])
            .output()
            .unwrap()
    };

    let first = run_json("0");
    assert!(
        first.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first_report: Value = serde_json::from_slice(&first.stdout).unwrap();
    let first_warnings = first_report["warnings"].as_array().unwrap();
    assert_eq!(first_warnings.len(), 1);
    assert_eq!(first_warnings[0]["kind"], "walkError");
    assert!(
        first_warnings[0]["path"]
            .as_str()
            .unwrap()
            .ends_with("target/denied")
    );
    assert_eq!(first_report["projects"][0]["candidates"][0]["bytes"], 4096);

    let second = run_json("0");
    assert!(second.status.success());
    let second_report: Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(second_report["warnings"], first_report["warnings"]);

    let filtered = run_json("4097");
    assert_eq!(filtered.status.code(), Some(3));
    let filtered_report: Value = serde_json::from_slice(&filtered.stdout).unwrap();
    assert_eq!(filtered_report["summary"]["candidates"], 0);
    assert_eq!(filtered_report["warnings"], first_report["warnings"]);

    Command::cargo_bin("rclean")
        .unwrap()
        .args(["scan", temp.path().to_str().unwrap(), "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Warnings during scan:"))
        .stdout(predicate::str::contains("Results may be incomplete."));
}
