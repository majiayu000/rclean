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
    validate_for_deletion(&dir).expect("real directory must validate");
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
    let err = validate_for_deletion(&link)
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

    let err = validate_for_deletion(&hardlink)
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

    let err = validate_for_deletion(&junction)
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
    let err = validate_for_deletion(&missing)
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
    let err = validate_for_deletion(&file)
        .expect_err("file must be rejected")
        .to_string();
    assert!(
        err.contains("no longer a directory"),
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

    let result = delete_selected(&selected, true, None).unwrap();
    assert!(result.cleaned.is_empty());
    assert_eq!(result.failed.len(), 1);
    assert!(real.is_dir(), "symlink target must not be deleted");
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

    let result = delete_selected(&selected, true, None).unwrap();
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

    let result = delete_selected(&selected, true, Some(&mut logger)).unwrap();

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

    let result = delete_selected(&selected, true, None).unwrap();

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

    let err = validate_for_deletion_with_rule(&sessions, Some("go.build_cache"))
        .expect_err("Codex session history must never be cleanable")
        .to_string();

    assert!(
        err.contains("protected user data"),
        "unexpected error: {err}"
    );
}
