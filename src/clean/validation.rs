use std::fs;
use std::path::Path;
#[cfg(target_os = "macos")]
use std::process::Command;

use crate::error::CleanError;
use crate::rules;
use crate::scan::{is_protected_user_data_path, is_runtime_or_system_path};

use super::types::SelectedCandidate;

#[cfg(test)]
pub(super) fn validate_for_deletion(path: &Path) -> Result<(), CleanError> {
    validate_for_deletion_with_rule(path, None)
}

pub(super) fn validate_candidate_for_deletion(
    candidate: &SelectedCandidate,
) -> Result<(), CleanError> {
    validate_for_deletion_with_rule(&candidate.path, Some(&candidate.rule_id))
}

pub(super) fn validate_for_deletion_with_rule(
    path: &Path,
    rule_id: Option<&str>,
) -> Result<(), CleanError> {
    let metadata = fs::symlink_metadata(path).map_err(|err| {
        CleanError::Generic(format!(
            "{} no longer exists or cannot be read: {err}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(CleanError::Generic(format!(
            "refusing to delete {}: path is now a symlink",
            path.display()
        )));
    }
    if !metadata.is_dir() {
        return Err(CleanError::Generic(format!(
            "refusing to delete {}: path is no longer a directory",
            path.display()
        )));
    }

    let canonical = path.canonicalize().map_err(|err| {
        CleanError::Generic(format!("failed to canonicalize {}: {err}", path.display()))
    })?;
    if is_protected_user_data_path(&canonical) {
        return Err(CleanError::Generic(format!(
            "refusing to delete {}: resolves to protected user data",
            path.display()
        )));
    }
    let allowed_global_rule = rule_id.is_some_and(rules::is_global_rule);
    if is_runtime_or_system_path(&canonical) && !allowed_global_rule {
        return Err(CleanError::Generic(format!(
            "refusing to delete {}: resolves to a protected runtime or system path",
            path.display()
        )));
    }
    if rule_id.is_some_and(requires_closed_process_gate) {
        ensure_no_open_files(path)?;
    }

    Ok(())
}

fn requires_closed_process_gate(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "macos.chrome_code_sign_clone"
            | "macos.remem_dry_run_tmp"
            | "app.lark_cache"
            | "app.lark_update"
            | "app.electron_cache"
            | "editor.vscode_cache"
            | "editor.cursor_cache"
            | "chrome.opt_guide_model"
    )
}

#[cfg(target_os = "macos")]
fn ensure_no_open_files(path: &Path) -> Result<(), CleanError> {
    let output = Command::new("lsof")
        .args(["-t", "+D"])
        .arg(path)
        .output()
        .map_err(|err| {
            CleanError::Generic(format!(
                "failed to check open files for {}: {err}",
                path.display()
            ))
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    open_file_check_result(path, output.status.success(), &stdout, &stderr)
}

#[cfg(any(target_os = "macos", test))]
fn open_file_check_result(
    path: &Path,
    success: bool,
    stdout: &str,
    stderr: &str,
) -> Result<(), CleanError> {
    if !stdout.trim().is_empty() {
        return Err(CleanError::Generic(format!(
            "refusing to delete {}: files are open by process ids {}",
            path.display(),
            stdout.split_whitespace().collect::<Vec<_>>().join(",")
        )));
    }
    if !success && !stderr.trim().is_empty() {
        return Err(CleanError::Generic(format!(
            "failed to check open files for {}: {}",
            path.display(),
            stderr.trim()
        )));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn ensure_no_open_files(_path: &Path) -> Result<(), CleanError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn closed_process_gate_applies_to_app_and_temp_rules() {
        assert!(requires_closed_process_gate("macos.chrome_code_sign_clone"));
        assert!(requires_closed_process_gate("macos.remem_dry_run_tmp"));
        assert!(requires_closed_process_gate("app.lark_update"));
        assert!(!requires_closed_process_gate("node.node_modules"));
    }

    #[test]
    fn open_file_check_rejects_open_process_ids() {
        let err = open_file_check_result(&PathBuf::from("/tmp/cache"), true, "123\n456\n", "")
            .unwrap_err();
        assert!(err.to_string().contains("123,456"));
    }

    #[test]
    fn open_file_check_allows_no_open_files_lsof_exit_one() {
        open_file_check_result(&PathBuf::from("/tmp/cache"), false, "", "").unwrap();
    }

    #[test]
    fn open_file_check_reports_lsof_errors() {
        let err =
            open_file_check_result(&PathBuf::from("/tmp/cache"), false, "", "permission denied")
                .unwrap_err();
        assert!(err.to_string().contains("permission denied"));
    }
}
