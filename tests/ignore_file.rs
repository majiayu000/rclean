use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

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
