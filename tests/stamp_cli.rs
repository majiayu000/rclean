use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn stamp_sweep_writes_plan_for_cargo_and_npm() {
    let temp = TempDir::new().unwrap();

    let npm = temp.path().join("npm-app");
    std::fs::create_dir(&npm).unwrap();
    std::fs::write(npm.join("package.json"), "{}").unwrap();
    std::fs::create_dir(npm.join("node_modules")).unwrap();
    std::fs::write(npm.join("node_modules").join("blob"), "abc").unwrap();

    let cargo = temp.path().join("rust-app");
    std::fs::create_dir(&cargo).unwrap();
    std::fs::write(
        cargo.join("Cargo.toml"),
        "[package]\nname='x'\nversion='0.1.0'\n",
    )
    .unwrap();
    std::fs::create_dir(cargo.join("target")).unwrap();
    std::fs::write(cargo.join("target").join("blob"), "abc").unwrap();

    let mut stamp = Command::cargo_bin("rclean").unwrap();
    stamp
        .args(["stamp", temp.path().to_str().unwrap(), "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stamped: 2"));

    assert!(npm.join("node_modules").join(".rclean-stamp").exists());
    assert!(cargo.join("target").join(".rclean-stamp").exists());

    let plan = temp.path().join("sweep-plan.json");
    let mut sweep = Command::cargo_bin("rclean").unwrap();
    sweep
        .args([
            "stamp",
            temp.path().to_str().unwrap(),
            "--sweep",
            "--write-plan",
            plan.to_str().unwrap(),
            "--min-size",
            "0",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Sweep candidates: 2"));

    let plan_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&plan).unwrap()).unwrap();
    assert_eq!(plan_json["selected"].as_array().unwrap().len(), 2);
    assert!(
        std::fs::read_to_string(plan)
            .unwrap()
            .contains("node.node_modules")
    );
}
