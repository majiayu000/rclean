use std::fs;
use std::path::Path;

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

    Ok(())
}
