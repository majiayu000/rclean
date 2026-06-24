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

        let outcome = if is_go_modcache_rule(&candidate.rule_id) {
            clean_go_modcache(candidate).map_err(|err| err.to_string())
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

        if is_go_modcache_rule(&candidate.rule_id) {
            let reason = "Go module cache cleanup must run `go clean -modcache`; graveyard mode cannot preserve that tool-managed operation"
                .to_string();
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

fn is_go_modcache_rule(rule_id: &str) -> bool {
    matches!(rule_id, "go.module_cache" | "go.module_download_cache")
}

fn delete_mode_for_candidate(candidate: &SelectedCandidate, permanent: bool) -> DeleteAuditMode {
    if is_go_modcache_rule(&candidate.rule_id) {
        DeleteAuditMode::GoModcache
    } else if permanent {
        DeleteAuditMode::Permanent
    } else {
        DeleteAuditMode::Trash
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
mod tests {
    use super::*;
    use crate::model::{Category, Safety};
    use tempfile::TempDir;

    #[test]
    fn resolves_go_modcache_from_root_candidate() {
        let candidate = SelectedCandidate {
            id: None,
            path: PathBuf::from("/Users/me/go/pkg/mod"),
            bytes: 0,
            rule_id: "go.module_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            risk_score: 0.0,
        };
        assert_eq!(
            go_modcache_path(&candidate),
            Some(PathBuf::from("/Users/me/go/pkg/mod"))
        );
    }

    #[test]
    fn resolves_go_modcache_from_legacy_download_candidate() {
        let candidate = SelectedCandidate {
            id: None,
            path: PathBuf::from("/Users/me/go/pkg/mod/cache/download"),
            bytes: 0,
            rule_id: "go.module_download_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            risk_score: 0.0,
        };
        assert_eq!(
            go_modcache_path(&candidate),
            Some(PathBuf::from("/Users/me/go/pkg/mod"))
        );
    }

    #[test]
    fn fake_go_modcache_clean_success() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let module_cache = temp.path().join("go").join("pkg").join("mod");
        std::fs::create_dir_all(module_cache.join("cache").join("download"))?;
        let fake_go = write_fake_go_success(&temp.path().join("bin"))?;
        let candidate = go_modcache_candidate(module_cache.clone());

        clean_go_modcache_with_tool(
            &candidate,
            fake_go_program(&fake_go)?,
            Duration::from_secs(1),
        )?;

        let output = std::fs::read_to_string(fake_go_output(&fake_go))?;
        assert!(output.contains("clean"));
        assert!(output.contains("-modcache"));
        assert!(output.contains(&format!("GOMODCACHE={}", module_cache.display())));
        Ok(())
    }

    #[test]
    fn fake_go_modcache_clean_nonzero_is_explicit_failure() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = TempDir::new()?;
        let module_cache = temp.path().join("go").join("pkg").join("mod");
        std::fs::create_dir_all(module_cache.join("cache").join("download"))?;
        let fake_go = write_fake_go_nonzero(&temp.path().join("bin"))?;
        let candidate = go_modcache_candidate(module_cache.clone());

        let err = expected_clean_error(
            clean_go_modcache_with_tool(
                &candidate,
                fake_go_program(&fake_go)?,
                Duration::from_secs(1),
            ),
            "nonzero fake go must fail",
        )?;

        assert!(err.contains("go clean -modcache failed"));
        assert!(err.contains(&module_cache.display().to_string()));
        assert!(err.contains("exited"));
        assert!(err.contains("permission denied"));
        Ok(())
    }

    #[test]
    fn fake_go_modcache_clean_timeout_is_explicit_failure() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = TempDir::new()?;
        let module_cache = temp.path().join("go").join("pkg").join("mod");
        std::fs::create_dir_all(module_cache.join("cache").join("download"))?;
        let fake_go = write_fake_go_timeout(&temp.path().join("bin"))?;
        let candidate = go_modcache_candidate(module_cache.clone());

        let err = expected_clean_error(
            clean_go_modcache_with_tool(
                &candidate,
                fake_go_program(&fake_go)?,
                Duration::from_millis(50),
            ),
            "timed out fake go must fail",
        )?;

        assert!(err.contains("go clean -modcache failed"));
        assert!(err.contains(&module_cache.display().to_string()));
        assert!(err.contains("timed out"));
        Ok(())
    }

    fn go_modcache_candidate(path: PathBuf) -> SelectedCandidate {
        SelectedCandidate {
            id: None,
            path,
            bytes: 0,
            rule_id: "go.module_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            risk_score: 0.0,
        }
    }

    fn fake_go_output(fake_go: &Path) -> PathBuf {
        fake_go.with_file_name("out.txt")
    }

    fn fake_go_program(fake_go: &Path) -> std::io::Result<&str> {
        fake_go.to_str().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "fake go path should be UTF-8",
            )
        })
    }

    fn expected_clean_error(
        result: Result<(), CleanError>,
        message: &'static str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        match result {
            Ok(()) => Err(std::io::Error::other(message).into()),
            Err(err) => Ok(err.to_string()),
        }
    }

    #[cfg(unix)]
    fn write_fake_go_success(parent: &Path) -> std::io::Result<PathBuf> {
        write_fake_go(
            parent,
            "#!/bin/sh\nout=\"$(dirname \"$0\")/out.txt\"\nprintf '%s\\n' \"$@\" > \"$out\"\nprintf 'GOMODCACHE=%s\\n' \"$GOMODCACHE\" >> \"$out\"\n",
        )
    }

    #[cfg(unix)]
    fn write_fake_go_nonzero(parent: &Path) -> std::io::Result<PathBuf> {
        write_fake_go(
            parent,
            "#!/bin/sh\nprintf 'permission denied\\n' >&2\nexit 23\n",
        )
    }

    #[cfg(unix)]
    fn write_fake_go_timeout(parent: &Path) -> std::io::Result<PathBuf> {
        write_fake_go(
            parent,
            "#!/bin/sh\nprintf 'started\\n' >&2\nwhile :; do :; done\n",
        )
    }

    #[cfg(unix)]
    fn write_fake_go(parent: &Path, script: &str) -> std::io::Result<PathBuf> {
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(parent)?;
        let fake_go = parent.join("go");
        std::fs::write(&fake_go, script)?;
        let mut permissions = std::fs::metadata(&fake_go)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake_go, permissions)?;
        Ok(fake_go)
    }

    #[cfg(windows)]
    fn write_fake_go_success(parent: &Path) -> std::io::Result<PathBuf> {
        write_fake_go(
            parent,
            "@echo off\r\nset OUT=%~dp0out.txt\r\n> \"%OUT%\" (\r\n  echo %1\r\n  echo %2\r\n  echo GOMODCACHE=%GOMODCACHE%\r\n)\r\n",
        )
    }

    #[cfg(windows)]
    fn write_fake_go_nonzero(parent: &Path) -> std::io::Result<PathBuf> {
        write_fake_go(
            parent,
            "@echo off\r\necho permission denied 1>&2\r\nexit /b 23\r\n",
        )
    }

    #[cfg(windows)]
    fn write_fake_go_timeout(parent: &Path) -> std::io::Result<PathBuf> {
        write_fake_go(
            parent,
            "@echo off\r\necho started 1>&2\r\n:loop\r\ngoto loop\r\n",
        )
    }

    #[cfg(windows)]
    fn write_fake_go(parent: &Path, script: &str) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(parent)?;
        let fake_go = parent.join("go.cmd");
        std::fs::write(&fake_go, script)?;
        Ok(fake_go)
    }
}
