use std::fs;

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

        let outcome = if permanent {
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
