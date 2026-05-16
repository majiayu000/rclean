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
