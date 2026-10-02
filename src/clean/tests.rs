use std::fs;
use std::path::PathBuf;
#[cfg(windows)]
use std::process::Command;

use tempfile::TempDir;

use super::audit::{DeleteAuditLogger, validate_audit_log_path};
use super::deletion::delete_selected;
use super::roots::check_broad_roots;
use super::selection::parse_selection;
use super::types::SelectedCandidate;
use super::validation::{validate_for_deletion, validate_for_deletion_with_rule};
use crate::model::Safety;

#[test]
fn parses_interactive_selection() {
    assert_eq!(parse_selection("", 5).unwrap(), Vec::<usize>::new());
    assert_eq!(parse_selection("a", 3).unwrap(), vec![0, 1, 2]);
    assert_eq!(parse_selection("1,3-4,3", 5).unwrap(), vec![0, 2, 3]);
    assert!(parse_selection("0", 3).is_err());
    assert!(parse_selection("4", 3).is_err());
    assert!(parse_selection("3-1", 3).is_err());
}

#[test]
fn check_broad_roots_rejects_root_slash() {
    let err = check_broad_roots(&[PathBuf::from("/")])
        .expect_err("/ must be rejected as broad")
        .to_string();
    assert!(err.contains("broad root"), "unexpected error: {err}");
}

#[test]
fn check_broad_roots_rejects_etc() {
    let err = check_broad_roots(&[PathBuf::from("/etc")])
        .expect_err("/etc must be rejected as broad")
        .to_string();
    assert!(err.contains("broad root"), "unexpected error: {err}");
}

#[test]
fn check_broad_roots_accepts_normal_project_path() {
    let temp = TempDir::new().unwrap();
    check_broad_roots(&[temp.path().to_path_buf()])
        .expect("a normal tempdir path must not be flagged as broad");
}

#[test]
fn validate_accepts_real_directory() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("artifact");
    fs::create_dir(&dir).unwrap();
    validate_for_deletion(&dir, &test_roots(&temp)).expect("real directory must validate");
}

#[test]
fn validate_rejects_symlink() {
    let temp = TempDir::new().unwrap();
    let real = temp.path().join("real");
    let link = temp.path().join("link");
    fs::create_dir(&real).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real, &link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&real, &link).unwrap();
    let err = validate_for_deletion(&link, &test_roots(&temp))
        .expect_err("symlink must be rejected")
        .to_string();
    assert!(err.contains("symlink"), "unexpected error: {err}");
}

#[test]
#[cfg(unix)]
fn validate_rejects_hardlinked_file_before_directory_check() {
    let temp = TempDir::new().unwrap();
    let original = temp.path().join("original");
    let hardlink = temp.path().join("artifact");
    fs::write(&original, "content").unwrap();
    fs::hard_link(&original, &hardlink).unwrap();

    let err = validate_for_deletion(&hardlink, &test_roots(&temp))
        .expect_err("hardlinked regular file must be rejected")
        .to_string();

    assert!(err.contains("hardlinked file"), "unexpected error: {err}");
}

#[test]
#[cfg(windows)]
fn validate_rejects_junction() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("target");
    let junction = temp.path().join("artifact");
    fs::create_dir(&target).unwrap();
    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&junction)
        .arg(&target)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "mklink failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let err = validate_for_deletion(&junction, &test_roots(&temp))
        .expect_err("junction must be rejected")
        .to_string();

    assert!(
        err.contains("symlink") || err.contains("junction") || err.contains("reparse point"),
        "unexpected error: {err}"
    );
}

#[test]
fn validate_rejects_missing_path() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("missing");
    let err = validate_for_deletion(&missing, &test_roots(&temp))
        .expect_err("missing path must be rejected")
        .to_string();
    assert!(
        err.contains("no longer exists") || err.contains("cannot be read"),
        "unexpected error: {err}"
    );
}

#[test]
fn validate_rejects_file() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("file");
    fs::write(&file, b"x").unwrap();
    let err = validate_for_deletion(&file, &test_roots(&temp))
        .expect_err("file must be rejected")
        .to_string();
    assert!(
        err.contains("no longer a directory"),
        "unexpected error: {err}"
    );
}

#[test]
fn validate_rejects_docker_daemon_storage() {
    let temp = TempDir::new().unwrap();
    let project = temp
        .path()
        .join("var")
        .join("lib")
        .join("docker")
        .join("project");
    let target = project.join("target");
    fs::create_dir_all(&target).unwrap();
    fs::write(project.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
    fs::write(target.join("placeholder"), b"x").unwrap();

    let err = validate_for_deletion_with_rule(&target, Some("rust.target"), &test_roots(&temp))
        .expect_err("Docker daemon storage must not pass final delete validation")
        .to_string();

    assert!(
        err.contains("Docker daemon storage"),
        "unexpected error: {err}"
    );
}

#[test]
fn delete_selected_skips_swapped_symlink_target() {
    let temp = TempDir::new().unwrap();
    let real = temp.path().join("real");
    let candidate_path = temp.path().join("artifact");
    fs::create_dir(&real).unwrap();
    fs::create_dir(&candidate_path).unwrap();

    let selected = vec![SelectedCandidate {
        id: None,
        path: candidate_path.clone(),
        bytes: 0,
        rule_id: "test".to_string(),
        category: crate::model::Category::Build,
        safety: Safety::Safe,
        requires_sudo: false,
        risk_score: 0.0,
    }];

    // TOCTOU: replace the candidate directory with a symlink between scan and delete.
    fs::remove_dir(&candidate_path).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real, &candidate_path).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&real, &candidate_path).unwrap();

    let result = delete_selected(&selected, &test_roots(&temp), true, None).unwrap();
    assert!(result.cleaned.is_empty());
    assert_eq!(result.failed.len(), 1);
    assert!(real.is_dir(), "symlink target must not be deleted");
}

#[test]
fn delete_selected_rejects_ancestor_swapped_outside_scan_root() {
    use clap::Parser;

    let mut modes = vec!["permanent", "trash"];
    if cfg!(feature = "graveyard") {
        modes.push("graveyard");
    }
    for mode in modes {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("scan");
        let project = root.join("project");
        fs::create_dir_all(project.join("node_modules")).unwrap();
        fs::write(project.join("package.json"), "{}").unwrap();
        fs::write(project.join("node_modules/blob"), b"scanned").unwrap();

        let cli = crate::cli::Cli::parse_from(["rclean", "clean", "--all", "--min-size", "0"]);
        let Some(crate::cli::Commands::Clean(args)) = cli.command else {
            panic!("expected clean args");
        };
        let report = crate::scan::scan(&[root], &args.common.to_scan_options().unwrap()).unwrap();
        let super::types::SelectionOutcome::Confirmed(selected) =
            super::selection::select_candidates(&report, &args).unwrap()
        else {
            panic!("expected confirmed selection");
        };
        assert_eq!(selected.len(), 1);

        let outside = temp.path().join("outside");
        let outside_artifact = outside.join("node_modules");
        fs::create_dir_all(&outside_artifact).unwrap();
        fs::write(outside_artifact.join("blob"), b"must remain").unwrap();
        fs::rename(&project, temp.path().join("original_project")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &project).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&outside, &project).unwrap();
        assert!(fs::symlink_metadata(&selected[0].path).unwrap().is_dir());

        let audit_path = temp.path().join("audit.jsonl");
        let mut logger = DeleteAuditLogger::new(&audit_path).unwrap();
        let result = match mode {
            "permanent" => {
                delete_selected(&selected, &report.roots, true, Some(&mut logger)).unwrap()
            }
            "trash" => delete_selected(&selected, &report.roots, false, Some(&mut logger)).unwrap(),
            #[cfg(feature = "graveyard")]
            "graveyard" => {
                let yard = crate::graveyard::Graveyard::open(temp.path().join("graveyard"));
                let result = super::deletion::delete_selected_into_graveyard(
                    &selected,
                    &report.roots,
                    &yard,
                    Some(&mut logger),
                )
                .unwrap();
                assert!(
                    !yard.root().exists(),
                    "rejected candidate must not create a grave"
                );
                result
            }
            _ => unreachable!(),
        };

        assert!(
            result.cleaned.is_empty(),
            "{mode} must refuse the escaped path"
        );
        assert_eq!(result.failed.len(), 1);
        assert!(result.failed[0].1.contains("outside the scan roots"));
        assert_eq!(
            fs::read(outside_artifact.join("blob")).unwrap(),
            b"must remain"
        );
        let entry: serde_json::Value =
            serde_json::from_str(fs::read_to_string(audit_path).unwrap().trim()).unwrap();
        assert_eq!(entry["result"], "failed");
        assert_eq!(entry["mode"], mode);
        assert!(
            entry["reason"]
                .as_str()
                .unwrap()
                .contains("outside the scan roots")
        );
    }
}

#[test]
fn delete_selected_accepts_candidate_inside_any_scan_root() {
    let temp = TempDir::new().unwrap();
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    let artifact = second.join("node_modules");
    fs::create_dir(&first).unwrap();
    fs::create_dir_all(&artifact).unwrap();
    let selected = vec![SelectedCandidate {
        id: None,
        path: artifact.clone(),
        bytes: 0,
        rule_id: "node.node_modules".to_string(),
        category: crate::model::Category::Deps,
        safety: Safety::Safe,
        requires_sudo: false,
        risk_score: 0.0,
    }];
    let roots = vec![
        first.canonicalize().unwrap().display().to_string(),
        second.canonicalize().unwrap().display().to_string(),
    ];

    let result = delete_selected(&selected, &roots, true, None).unwrap();

    assert!(result.failed.is_empty());
    assert_eq!(result.cleaned.len(), 1);
    assert!(!artifact.exists());
}

#[test]
fn validate_rejects_directory_without_scan_roots() {
    let temp = TempDir::new().unwrap();
    let artifact = temp.path().join("artifact");
    fs::create_dir(&artifact).unwrap();

    let err = validate_for_deletion(&artifact, &[])
        .unwrap_err()
        .to_string();

    assert!(
        err.contains("outside the scan roots"),
        "unexpected error: {err}"
    );
    assert!(artifact.is_dir());
}

#[test]
fn delete_selected_skips_swapped_file_candidate() {
    let temp = TempDir::new().unwrap();
    let candidate_path = temp.path().join("artifact");
    fs::create_dir(&candidate_path).unwrap();

    let selected = vec![SelectedCandidate {
        id: None,
        path: candidate_path.clone(),
        bytes: 0,
        rule_id: "test".to_string(),
        category: crate::model::Category::Build,
        safety: Safety::Safe,
        requires_sudo: false,
        risk_score: 0.0,
    }];

    // TOCTOU: replace the candidate directory with a regular file between
    // scan and delete. Final validation must fail before remove_dir_all runs.
    fs::remove_dir(&candidate_path).unwrap();
    fs::write(&candidate_path, b"not a directory").unwrap();

    let result = delete_selected(&selected, &test_roots(&temp), true, None).unwrap();
    assert!(result.cleaned.is_empty());
    assert_eq!(result.failed.len(), 1);
    assert!(
        candidate_path.is_file(),
        "replacement file must not be deleted"
    );
}

#[test]
fn delete_selected_logs_validation_failure() {
    let temp = TempDir::new().unwrap();
    let audit_path = temp.path().join("audit.jsonl");
    let mut logger = DeleteAuditLogger::new(&audit_path).unwrap();
    let missing = temp.path().join("missing");
    let selected = vec![SelectedCandidate {
        id: None,
        path: missing,
        bytes: 0,
        rule_id: "test".to_string(),
        category: crate::model::Category::Build,
        safety: Safety::Safe,
        requires_sudo: false,
        risk_score: 0.0,
    }];

    let result = delete_selected(&selected, &test_roots(&temp), true, Some(&mut logger)).unwrap();

    assert!(result.cleaned.is_empty());
    assert_eq!(result.failed.len(), 1);
    let raw = fs::read_to_string(audit_path).unwrap();
    let entry: serde_json::Value = serde_json::from_str(raw.trim()).unwrap();
    assert_eq!(entry["result"], "failed");
    assert_eq!(entry["mode"], "permanent");
    assert!(
        entry["reason"]
            .as_str()
            .unwrap()
            .contains("no longer exists")
    );
}

#[test]
fn delete_selected_refuses_requires_sudo_candidate_before_deletion() {
    let temp = TempDir::new().unwrap();
    let candidate_path = temp
        .path()
        .join("Library")
        .join("Application Support")
        .join("com.apple.idleassetsd");
    fs::create_dir_all(&candidate_path).unwrap();
    fs::write(candidate_path.join("blob"), b"x").unwrap();
    let selected = vec![SelectedCandidate {
        id: None,
        path: candidate_path.clone(),
        bytes: 1,
        rule_id: "apple.idleassetsd".to_string(),
        category: crate::model::Category::Cache,
        safety: Safety::ReportOnly,
        requires_sudo: true,
        risk_score: 0.0,
    }];

    let result = delete_selected(&selected, &test_roots(&temp), true, None).unwrap();

    assert!(result.cleaned.is_empty());
    assert_eq!(result.failed.len(), 1);
    assert!(candidate_path.exists(), "requires-sudo path must remain");
    assert!(result.failed[0].1.contains("will not run sudo"));
}

#[test]
fn validate_audit_log_path_rejects_selected_descendant() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let candidate_path = temp.path().join("node_modules");
    fs::create_dir(&candidate_path)?;
    let selected = vec![SelectedCandidate {
        id: None,
        path: candidate_path.clone(),
        bytes: 0,
        rule_id: "node.node_modules".to_string(),
        category: crate::model::Category::Deps,
        safety: Safety::Safe,
        requires_sudo: false,
        risk_score: 0.0,
    }];

    let err = match validate_audit_log_path(&candidate_path.join("audit.jsonl"), &selected) {
        Ok(()) => {
            return Err(std::io::Error::other(
                "audit log inside selected candidate must be rejected",
            )
            .into());
        }
        Err(err) => err.to_string(),
    };

    assert!(err.contains("audit log"), "unexpected error: {err}");
    assert!(
        err.contains("selected candidate"),
        "unexpected error: {err}"
    );
    Ok(())
}

#[test]
fn validate_rejects_codex_sessions_even_for_global_rule() {
    let temp = TempDir::new().unwrap();
    let sessions = temp.path().join(".codex").join("sessions");
    fs::create_dir_all(&sessions).unwrap();

    let err =
        validate_for_deletion_with_rule(&sessions, Some("go.build_cache"), &test_roots(&temp))
            .expect_err("Codex session history must never be cleanable")
            .to_string();

    assert!(
        err.contains("protected user data"),
        "unexpected error: {err}"
    );
}

fn test_roots(temp: &TempDir) -> Vec<String> {
    vec![temp.path().canonicalize().unwrap().display().to_string()]
}
