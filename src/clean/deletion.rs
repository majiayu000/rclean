use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::CleanError;

use super::types::{CleanResult, SelectedCandidate};
use super::validation::validate_candidate_for_deletion;

pub fn delete_selected(
    selected: &[SelectedCandidate],
    permanent: bool,
) -> Result<CleanResult, CleanError> {
    let mut result = CleanResult::default();

    for candidate in selected {
        if let Err(err) = validate_candidate_for_deletion(candidate) {
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
            Ok(()) => result.cleaned.push(candidate.clone()),
            Err(err) => result.failed.push((candidate.clone(), err)),
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
) -> Result<CleanResult, CleanError> {
    use crate::graveyard::GraveInput;

    let tool_version = env!("CARGO_PKG_VERSION");
    let mut result = CleanResult::default();

    for candidate in selected {
        if let Err(err) = validate_candidate_for_deletion(candidate) {
            result.failed.push((candidate.clone(), err.to_string()));
            continue;
        }

        if is_go_modcache_rule(&candidate.rule_id) {
            result.failed.push((
                candidate.clone(),
                "Go module cache cleanup must run `go clean -modcache`; graveyard mode cannot preserve that tool-managed operation"
                    .to_string(),
            ));
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
            Ok(_) => result.cleaned.push(candidate.clone()),
            Err(err) => result.failed.push((candidate.clone(), err.to_string())),
        }
    }

    Ok(result)
}

fn is_go_modcache_rule(rule_id: &str) -> bool {
    matches!(rule_id, "go.module_cache" | "go.module_download_cache")
}

fn clean_go_modcache(candidate: &SelectedCandidate) -> Result<(), CleanError> {
    let modcache = go_modcache_path(candidate).ok_or_else(|| {
        CleanError::Generic(format!(
            "{} is not inside a Go module cache layout",
            candidate.path.display()
        ))
    })?;
    let output = Command::new("go")
        .args(["clean", "-modcache"])
        .env("GOMODCACHE", &modcache)
        .output()
        .map_err(|err| CleanError::Generic(format!("failed to run go clean -modcache: {err}")))?;
    if !output.status.success() {
        return Err(CleanError::Generic(format!(
            "go clean -modcache failed for {}: {}",
            modcache.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

fn go_modcache_path(candidate: &SelectedCandidate) -> Option<PathBuf> {
    if candidate.rule_id == "go.module_cache" {
        return Some(candidate.path.clone());
    }
    go_modcache_from_download_path(&candidate.path)
}

fn go_modcache_from_download_path(path: &Path) -> Option<PathBuf> {
    if path.file_name()?.to_str()? != "download" {
        return None;
    }
    let cache = path.parent()?;
    if cache.file_name()?.to_str()? != "cache" {
        return None;
    }
    let modcache = cache.parent()?;
    if modcache.file_name()?.to_str()? != "mod" {
        return None;
    }
    Some(modcache.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Category, Safety};

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
}
