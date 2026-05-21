use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn user_rule_matches_custom_artifact_directory() {
    // A custom Makefile-driven build dir that no builtin rule covers.
    // The user rule names it `user.makefile_build`, requires a Makefile
    // marker, and marks it as `safe`. End-to-end: rclean scan --json
    // should emit it as a candidate.
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("Makefile"), "all:\n\techo build\n").unwrap();
    std::fs::create_dir(temp.path().join("my_build_x86")).unwrap();
    std::fs::write(temp.path().join("my_build_x86").join("a.o"), b"x").unwrap();
    std::fs::write(
        temp.path().join(".rclean.toml"),
        r#"
[[rule]]
id = "user.makefile_build"
name_glob = "my_build_*"
parent_markers = ["Makefile"]
category = "build"
safety = "safe"
why = "Custom Makefile build dir"
restore_hint = "make build"
"#,
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
            "\"ruleId\": \"user.makefile_build\"",
        ))
        .stdout(predicate::str::contains("\"safety\": \"safe\""));
}

#[test]
fn user_rule_action_plan_replays_successfully() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("Makefile"), "all:\n\techo build\n").unwrap();
    std::fs::create_dir(temp.path().join("my_build_x86")).unwrap();
    std::fs::write(temp.path().join("my_build_x86").join("a.o"), b"x").unwrap();
    std::fs::write(
        temp.path().join(".rclean.toml"),
        r#"
[[rule]]
id = "user.makefile_build"
name_glob = "my_build_*"
parent_markers = ["Makefile"]
category = "build"
safety = "safe"
"#,
    )
    .unwrap();
    let plan = temp.path().join("plan.json");

    Command::cargo_bin("rclean")
        .unwrap()
        .args([
            "scan",
            temp.path().to_str().unwrap(),
            "--write-plan",
            plan.to_str().unwrap(),
            "--min-size",
            "0",
        ])
        .assert()
        .success();

    Command::cargo_bin("rclean")
        .unwrap()
        .args(["clean", "--plan", plan.to_str().unwrap(), "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan: 1 candidates"))
        .stdout(predicate::str::contains("user.makefile_build"));

    assert!(temp.path().join("my_build_x86").exists());
}

#[test]
fn user_rule_skipped_when_parent_marker_missing() {
    // Same .rclean.toml as above, but no Makefile in the project — the
    // rule must not fire. Exit code 3 = zero candidates, that's the
    // success signal here.
    let temp = TempDir::new().unwrap();
    std::fs::create_dir(temp.path().join("my_build_x86")).unwrap();
    std::fs::write(temp.path().join("my_build_x86").join("a.o"), b"x").unwrap();
    std::fs::write(
        temp.path().join(".rclean.toml"),
        r#"
[[rule]]
id = "user.makefile_build"
name_glob = "my_build_*"
parent_markers = ["Makefile"]
category = "build"
safety = "safe"
"#,
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
        .code(3)
        .stdout(predicate::str::contains("\"ruleId\":").not());
}

#[test]
fn user_rule_with_blocked_safety_is_rejected_and_warned() {
    // SPEC §4.2: user rules may not declare safety=blocked. The rule must
    // be dropped at load time with a warning, but the scan itself should
    // still succeed.
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("Makefile"), "all:\n").unwrap();
    std::fs::create_dir(temp.path().join("bad_dir")).unwrap();
    std::fs::write(temp.path().join("bad_dir").join("a"), b"x").unwrap();
    std::fs::write(
        temp.path().join(".rclean.toml"),
        r#"
[[rule]]
id = "user.evil"
name_glob = "bad_*"
parent_markers = ["Makefile"]
category = "build"
safety = "blocked"
"#,
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
        // Rule was rejected → no candidates → exit 3.
        .code(3)
        .stderr(predicate::str::contains("safety=blocked"))
        .stdout(predicate::str::contains("\"ruleId\":").not());
}
