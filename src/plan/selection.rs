use std::collections::HashSet;

use crate::clean::SelectedCandidate;
use crate::model::{Candidate, Safety, ScanReport, Summary};

use super::id::generate_candidate_id;
use super::schema::PlanCandidate;

pub(super) fn collect_selected(report: &ScanReport, include_caution: bool) -> Vec<PlanCandidate> {
    report
        .projects
        .iter()
        .flat_map(|project| project.candidates.iter())
        .filter(|candidate| {
            !candidate.requires_sudo
                && (candidate.safety == Safety::Safe
                    || (include_caution && candidate.safety == Safety::Caution))
        })
        .map(to_plan_candidate)
        .collect()
}

#[cfg_attr(not(feature = "tui"), allow(dead_code))]
pub(super) fn collect_selected_paths(
    report: &ScanReport,
    selected: &[SelectedCandidate],
) -> Vec<PlanCandidate> {
    let selected_paths = selected
        .iter()
        .map(|candidate| candidate.path.display().to_string())
        .collect::<HashSet<_>>();

    report
        .projects
        .iter()
        .flat_map(|project| project.candidates.iter())
        .filter(|candidate| selected_paths.contains(&candidate.path) && !candidate.requires_sudo)
        .map(to_plan_candidate)
        .collect()
}

/// Build a `Summary` that reflects what is actually in `selected`, while
/// preserving the scan-wide accounting (`projects_scanned`,
/// `projects_with_candidates`) from the original report. Without this,
/// `ActionPlan.summary` would describe the entire scan instead of the
/// chosen-for-deletion subset, misleading downstream consumers.
pub(super) fn summarize_selected(selected: &[PlanCandidate], scan_summary: &Summary) -> Summary {
    let mut summary = Summary {
        projects_scanned: scan_summary.projects_scanned,
        projects_with_candidates: scan_summary.projects_with_candidates,
        ..Summary::default()
    };
    for candidate in selected {
        summary.candidates += 1;
        summary.total_bytes = summary.total_bytes.saturating_add(candidate.bytes);
        match candidate.safety {
            Safety::Safe => summary.safe_candidates += 1,
            Safety::Caution => summary.caution_candidates += 1,
            Safety::Blocked => summary.blocked_candidates += 1,
            Safety::ReportOnly => summary.report_only_candidates += 1,
            Safety::Unknown => {}
        }
    }
    summary
}

fn to_plan_candidate(candidate: &Candidate) -> PlanCandidate {
    PlanCandidate {
        id: generate_candidate_id(),
        path: candidate.path.clone(),
        rule_id: candidate.rule_id.clone(),
        bytes: candidate.bytes,
        safety: candidate.safety,
        requires_sudo: candidate.requires_sudo,
        category: candidate.category,
        risk_score: candidate.risk_score,
    }
}
