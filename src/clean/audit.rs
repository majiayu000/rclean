use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Serialize;

use crate::error::CleanError;

use super::types::SelectedCandidate;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteAuditMode {
    Trash,
    Permanent,
    Graveyard,
    GoModcache,
    PipCache,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteAuditStatus {
    Success,
    Failed,
    Skipped,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DeleteAuditEntry {
    pub timestamp: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub rule_id: String,
    pub permanent: bool,
    pub mode: DeleteAuditMode,
    pub result: DeleteAuditStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

pub struct DeleteAuditLogger {
    path: PathBuf,
    file: File,
}

pub fn validate_audit_log_path(
    path: &Path,
    selected: &[SelectedCandidate],
) -> Result<(), CleanError> {
    let audit_path = comparable_path(path, "audit log")?;
    for candidate in selected {
        let candidate_path = comparable_path(&candidate.path, "selected candidate")?;
        if audit_path == candidate_path || audit_path.starts_with(&candidate_path) {
            return Err(CleanError::Generic(format!(
                "audit log {} is inside selected candidate {}; choose a path outside cleaned trees",
                path.display(),
                candidate.path.display()
            )));
        }
    }
    Ok(())
}

fn comparable_path(path: &Path, label: &str) -> Result<PathBuf, CleanError> {
    let absolute = absolute_path(path, label)?;
    let mut current = absolute.as_path();
    let mut suffix = Vec::new();

    while !current.exists() {
        let Some(name) = current.file_name() else {
            return Err(CleanError::Generic(format!(
                "failed to resolve {label} path {}",
                path.display()
            )));
        };
        suffix.push(name.to_os_string());
        let Some(parent) = current.parent().filter(|parent| *parent != current) else {
            return Err(CleanError::Generic(format!(
                "failed to resolve {label} path {}",
                path.display()
            )));
        };
        current = parent;
    }

    let mut resolved = current.canonicalize().map_err(|source| {
        CleanError::Generic(format!(
            "failed to resolve {label} path {}: {source}",
            path.display()
        ))
    })?;
    for part in suffix.iter().rev() {
        resolved.push(part);
    }
    Ok(resolved)
}

fn absolute_path(path: &Path, label: &str) -> Result<PathBuf, CleanError> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let cwd = std::env::current_dir().map_err(|source| {
        CleanError::Generic(format!(
            "failed to resolve {label} path {}: {source}",
            path.display()
        ))
    })?;
    Ok(cwd.join(path))
}

impl DeleteAuditLogger {
    pub fn new(path: &Path) -> Result<Self, CleanError> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).map_err(|source| {
                CleanError::Generic(format!(
                    "failed to create audit log parent {}: {source}",
                    parent.display()
                ))
            })?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|source| {
                CleanError::Generic(format!(
                    "failed to open audit log {}: {source}",
                    path.display()
                ))
            })?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
        })
    }

    pub fn log(
        &mut self,
        candidate: &SelectedCandidate,
        mode: DeleteAuditMode,
        result: DeleteAuditStatus,
        reason: Option<String>,
    ) -> Result<(), CleanError> {
        let entry = DeleteAuditEntry {
            timestamp: Utc::now().to_rfc3339(),
            path: candidate.path.clone(),
            size_bytes: candidate.bytes,
            rule_id: candidate.rule_id.clone(),
            permanent: matches!(
                mode,
                DeleteAuditMode::Permanent
                    | DeleteAuditMode::GoModcache
                    | DeleteAuditMode::PipCache
            ),
            mode,
            result,
            reason,
        };
        let json = serde_json::to_string(&entry).map_err(|source| {
            CleanError::Generic(format!(
                "failed to serialize audit log entry for {}: {source}",
                candidate.path.display()
            ))
        })?;
        writeln!(self.file, "{json}").map_err(|source| {
            CleanError::Generic(format!(
                "failed to write audit log {}: {source}",
                self.path.display()
            ))
        })?;
        self.file.flush().map_err(|source| {
            CleanError::Generic(format!(
                "failed to flush audit log {}: {source}",
                self.path.display()
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{DeleteAuditMode, DeleteAuditStatus};

    #[test]
    fn serializes_base_audit_variants_as_snake_case() {
        for (mode, expected) in [
            (DeleteAuditMode::Trash, r#""trash""#),
            (DeleteAuditMode::Permanent, r#""permanent""#),
            (DeleteAuditMode::GoModcache, r#""go_modcache""#),
            (DeleteAuditMode::PipCache, r#""pip_cache""#),
        ] {
            assert_eq!(serde_json::to_string(&mode).unwrap(), expected);
        }

        for (status, expected) in [
            (DeleteAuditStatus::Success, r#""success""#),
            (DeleteAuditStatus::Failed, r#""failed""#),
        ] {
            assert_eq!(serde_json::to_string(&status).unwrap(), expected);
        }
    }

    #[cfg(feature = "graveyard")]
    #[test]
    fn serializes_graveyard_audit_variants_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&DeleteAuditMode::Graveyard).unwrap(),
            r#""graveyard""#
        );
        assert_eq!(
            serde_json::to_string(&DeleteAuditStatus::Skipped).unwrap(),
            r#""skipped""#
        );
    }
}
