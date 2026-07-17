use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::CleanError;
use crate::path_util::path_file_name;

use super::audit::{DeleteAuditLogger, DeleteAuditMode, DeleteAuditStatus};
use super::native_tool::{NativeToolCommand, run_native_tool};
use super::types::{CleanResult, SelectedCandidate};
use super::validation::validate_candidate_for_deletion;

const GO_CLEAN_MODCACHE_TIMEOUT: Duration = Duration::from_secs(60);
const PIP_CACHE_PURGE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy)]
enum NativeToolCleanup {
    GoModcache,
    PipCache,
}

pub fn delete_selected(
    selected: &[SelectedCandidate],
    permanent: bool,
    mut audit_logger: Option<&mut DeleteAuditLogger>,
) -> Result<CleanResult, CleanError> {
    let mut result = CleanResult::default();

    for candidate in selected {
        let mode = delete_mode_for_candidate(candidate, permanent);
        if let Err(err) = validate_candidate_for_deletion(candidate) {
            log_audit(
                &mut audit_logger,
                candidate,
                mode,
                DeleteAuditStatus::Failed,
                Some(err.to_string()),
            )?;
            result.failed.push((candidate.clone(), err.to_string()));
            continue;
        }

        let outcome = if let Some(cleanup) = native_tool_cleanup_for_rule(&candidate.rule_id) {
            if permanent {
                clean_native_tool(candidate, cleanup).map_err(|err| err.to_string())
            } else {
                Err(native_tool_requires_permanent_reason(cleanup, "trash"))
            }
        } else if permanent {
            fs::remove_dir_all(&candidate.path).map_err(|err| err.to_string())
        } else {
            trash::delete(&candidate.path).map_err(|err| err.to_string())
        };

        match outcome {
            Ok(()) => {
                log_audit(
                    &mut audit_logger,
                    candidate,
                    mode,
                    DeleteAuditStatus::Success,
                    None,
                )?;
                result.cleaned.push(candidate.clone());
            }
            Err(err) => {
                log_audit(
                    &mut audit_logger,
                    candidate,
                    mode,
                    DeleteAuditStatus::Failed,
                    Some(err.clone()),
                )?;
                result.failed.push((candidate.clone(), err));
            }
        }
    }

    Ok(result)
}

/// `--graveyard` delete path: validate each candidate, then bury it
/// in the per-user graveyard. Returns the same `CleanResult` shape as
/// `delete_selected` so callers can print one summary regardless of
/// delete mode.
///
/// Plan-origin candidates carry their ActionPlan v2 candidate id in
/// `SelectedCandidate::id`; direct scan selections leave it empty.
#[cfg(feature = "graveyard")]
pub fn delete_selected_into_graveyard(
    selected: &[SelectedCandidate],
    graveyard: &crate::graveyard::Graveyard,
    mut audit_logger: Option<&mut DeleteAuditLogger>,
) -> Result<CleanResult, CleanError> {
    use crate::graveyard::GraveInput;

    let tool_version = env!("CARGO_PKG_VERSION");
    let mut result = CleanResult::default();

    for candidate in selected {
        if let Err(err) = validate_candidate_for_deletion(candidate) {
            log_audit(
                &mut audit_logger,
                candidate,
                DeleteAuditMode::Graveyard,
                DeleteAuditStatus::Failed,
                Some(err.to_string()),
            )?;
            result.failed.push((candidate.clone(), err.to_string()));
            continue;
        }

        if let Some(cleanup) = native_tool_cleanup_for_rule(&candidate.rule_id) {
            let reason = native_tool_requires_permanent_reason(cleanup, "graveyard");
            log_audit(
                &mut audit_logger,
                candidate,
                DeleteAuditMode::Graveyard,
                DeleteAuditStatus::Skipped,
                Some(reason.clone()),
            )?;
            result.failed.push((candidate.clone(), reason));
            continue;
        }

        let category = candidate.category.to_string();
        let safety = candidate.safety.to_string();
        let input = GraveInput {
            original_path: &candidate.path,
            size_bytes: candidate.bytes,
            plan_id: candidate.id.clone(),
            rule_id: &candidate.rule_id,
            category: &category,
            safety_at_delete: &safety,
            risk_score_at_delete: candidate.risk_score,
            tool_version,
        };

        match graveyard.bury(input) {
            Ok(_) => {
                log_audit(
                    &mut audit_logger,
                    candidate,
                    DeleteAuditMode::Graveyard,
                    DeleteAuditStatus::Success,
                    None,
                )?;
                result.cleaned.push(candidate.clone());
            }
            Err(err) => {
                log_audit(
                    &mut audit_logger,
                    candidate,
                    DeleteAuditMode::Graveyard,
                    DeleteAuditStatus::Failed,
                    Some(err.to_string()),
                )?;
                result.failed.push((candidate.clone(), err.to_string()));
            }
        }
    }

    Ok(result)
}

fn native_tool_cleanup_for_rule(rule_id: &str) -> Option<NativeToolCleanup> {
    match rule_id {
        "go.module_cache" | "go.module_download_cache" => Some(NativeToolCleanup::GoModcache),
        "pip.cache" => Some(NativeToolCleanup::PipCache),
        _ => None,
    }
}

fn delete_mode_for_candidate(candidate: &SelectedCandidate, permanent: bool) -> DeleteAuditMode {
    if !permanent {
        return DeleteAuditMode::Trash;
    }

    match native_tool_cleanup_for_rule(&candidate.rule_id) {
        Some(NativeToolCleanup::GoModcache) => DeleteAuditMode::GoModcache,
        Some(NativeToolCleanup::PipCache) => DeleteAuditMode::PipCache,
        None => DeleteAuditMode::Permanent,
    }
}

fn log_audit(
    audit_logger: &mut Option<&mut DeleteAuditLogger>,
    candidate: &SelectedCandidate,
    mode: DeleteAuditMode,
    result: DeleteAuditStatus,
    reason: Option<String>,
) -> Result<(), CleanError> {
    if let Some(logger) = audit_logger {
        logger.log(candidate, mode, result, reason)?;
    }
    Ok(())
}

fn clean_go_modcache(candidate: &SelectedCandidate) -> Result<(), CleanError> {
    clean_go_modcache_with_tool(candidate, "go", GO_CLEAN_MODCACHE_TIMEOUT)
}

fn clean_native_tool(
    candidate: &SelectedCandidate,
    cleanup: NativeToolCleanup,
) -> Result<(), CleanError> {
    match cleanup {
        NativeToolCleanup::GoModcache => clean_go_modcache(candidate),
        NativeToolCleanup::PipCache => {
            clean_pip_cache_with_tool(candidate, "pip", PIP_CACHE_PURGE_TIMEOUT)
        }
    }
}

fn native_tool_requires_permanent_reason(cleanup: NativeToolCleanup, mode: &str) -> String {
    format!(
        "{} cleanup must run `{}`; {mode} mode cannot preserve that tool-managed operation; rerun with --permanent",
        cleanup.label(),
        cleanup.command()
    )
}

impl NativeToolCleanup {
    fn label(self) -> &'static str {
        match self {
            NativeToolCleanup::GoModcache => "Go module cache",
            NativeToolCleanup::PipCache => "pip cache",
        }
    }

    fn command(self) -> &'static str {
        match self {
            NativeToolCleanup::GoModcache => "go clean -modcache",
            NativeToolCleanup::PipCache => "pip cache purge",
        }
    }
}

fn clean_go_modcache_with_tool(
    candidate: &SelectedCandidate,
    program: &str,
    timeout: Duration,
) -> Result<(), CleanError> {
    let modcache = go_modcache_path(candidate).ok_or_else(|| {
        CleanError::Generic(format!(
            "{} is not inside a Go module cache layout",
            candidate.path.display()
        ))
    })?;
    let envs = [("GOMODCACHE", modcache.as_os_str())];
    run_native_tool(NativeToolCommand {
        program,
        args: &["clean", "-modcache"],
        envs: &envs,
        timeout,
    })
    .map_err(|err| {
        CleanError::Generic(format!(
            "go clean -modcache failed for {}: {err}",
            modcache.display()
        ))
    })
}

fn clean_pip_cache_with_tool(
    candidate: &SelectedCandidate,
    program: &str,
    timeout: Duration,
) -> Result<(), CleanError> {
    let envs = [("PIP_CACHE_DIR", candidate.path.as_os_str())];
    run_native_tool(NativeToolCommand {
        program,
        args: &["cache", "purge"],
        envs: &envs,
        timeout,
    })
    .map_err(|err| {
        CleanError::Generic(format!(
            "pip cache purge failed for {}: {err}",
            candidate.path.display()
        ))
    })
}

fn go_modcache_path(candidate: &SelectedCandidate) -> Option<PathBuf> {
    if candidate.rule_id == "go.module_cache" {
        return Some(candidate.path.clone());
    }
    go_modcache_from_download_path(&candidate.path)
}

fn go_modcache_from_download_path(path: &Path) -> Option<PathBuf> {
    if path_file_name(path)? != "download" {
        return None;
    }
    let cache = path.parent()?;
    if path_file_name(cache)? != "cache" {
        return None;
    }
    let modcache = cache.parent()?;
    if path_file_name(modcache)? != "mod" {
        return None;
    }
    Some(modcache.to_path_buf())
}

#[cfg(test)]
mod tests;
