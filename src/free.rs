//! `rclean free <size>` — goal-oriented cleanup (spec:
//! docs/specs/v0.2-best-ux.md §3.3 C1).
//!
//! Proposes the smallest set of `safe` candidates whose total meets the
//! requested reclaim target, preferring stale projects, and writes the
//! proposal as a reviewable ActionPlan. It never selects caution,
//! blocked, or report-only candidates, never widens the selection when
//! the target cannot be met, and never deletes anything itself — the
//! plan goes through `rclean clean --plan` with full revalidation.

use std::path::PathBuf;
use std::process::ExitCode;

use chrono::Utc;

use crate::clean::SelectedCandidate;
use crate::cli::FreeArgs;
use crate::error::RcleanError;
use crate::model::{Candidate, Safety, ScanReport, format_bytes};
use crate::{parse, plan, scan};

pub fn run(args: FreeArgs) -> Result<ExitCode, RcleanError> {
    let target = parse::parse_size(&args.target)?;
    let options = args.common.to_scan_options()?;
    let report = scan::scan(&args.common.paths_or_current_dir(), &options)?;

    let proposal = select_for_target(&report, target);

    if proposal.candidates.is_empty() {
        println!(
            "no safe candidates available; cannot free {}",
            format_bytes(target)
        );
        return Ok(ExitCode::from(3));
    }

    println!(
        "Proposed set to free {} (smallest safe set, stale projects first):",
        format_bytes(target)
    );
    for entry in &proposal.candidates {
        println!(
            "  - {} ({}, {}, risk {:.2}{})",
            entry.candidate.path,
            entry.candidate.rule_id,
            format_bytes(entry.candidate.bytes),
            entry.candidate.risk_score,
            match entry.candidate.staleness_days {
                Some(days) => format!(", untouched {days}d"),
                None => String::new(),
            }
        );
    }
    println!(
        "Total: {} of {} requested",
        format_bytes(proposal.total_bytes),
        format_bytes(target)
    );

    let plan_path = args
        .common
        .write_plan
        .clone()
        .unwrap_or_else(default_free_plan_path);
    let selected: Vec<SelectedCandidate> = proposal
        .candidates
        .iter()
        .map(|entry| to_selected(entry.candidate))
        .collect();
    plan::write_selected_action_plan(&report, &plan_path, &selected, "trash")?;
    println!("wrote action plan: {}", plan_path.display());
    println!(
        "review it, then run: rclean clean --plan {}",
        plan_path.display()
    );

    if proposal.total_bytes >= target {
        Ok(ExitCode::SUCCESS)
    } else {
        // Never widen the selection silently (U-29): the gap is stated
        // explicitly and the exit code says the target was not met.
        println!(
            "target not met: safe candidates cover {}, short by {}",
            format_bytes(proposal.total_bytes),
            format_bytes(target - proposal.total_bytes)
        );
        Ok(ExitCode::from(3))
    }
}

struct RankedCandidate<'a> {
    candidate: &'a Candidate,
}

struct Proposal<'a> {
    candidates: Vec<RankedCandidate<'a>>,
    total_bytes: u64,
}

/// Greedy selection over the spec ranking (staleness desc, risk asc,
/// size desc), followed by a reverse prune so the set stays minimal:
/// once the target is reached, any picked candidate the total can
/// spare is dropped, lowest-ranked first.
fn select_for_target(report: &ScanReport, target: u64) -> Proposal<'_> {
    let mut eligible: Vec<&Candidate> = report
        .projects
        .iter()
        .flat_map(|project| project.candidates.iter())
        .filter(|candidate| {
            candidate.safety == Safety::Safe && !candidate.requires_sudo && candidate.bytes > 0
        })
        .collect();

    eligible.sort_by(|a, b| {
        b.staleness_days
            .unwrap_or(0)
            .cmp(&a.staleness_days.unwrap_or(0))
            .then(a.risk_score.total_cmp(&b.risk_score))
            .then(b.bytes.cmp(&a.bytes))
    });

    let mut picked: Vec<&Candidate> = Vec::new();
    let mut total: u64 = 0;
    for candidate in eligible {
        if total >= target {
            break;
        }
        total += candidate.bytes;
        picked.push(candidate);
    }

    if total >= target {
        // Reverse prune: drop lowest-ranked picks the target can spare.
        let mut index = picked.len();
        while index > 0 {
            index -= 1;
            if total - picked[index].bytes >= target {
                total -= picked[index].bytes;
                picked.remove(index);
            }
        }
    }

    Proposal {
        candidates: picked
            .into_iter()
            .map(|candidate| RankedCandidate { candidate })
            .collect(),
        total_bytes: total,
    }
}

fn to_selected(candidate: &Candidate) -> SelectedCandidate {
    SelectedCandidate {
        id: None,
        path: PathBuf::from(&candidate.path),
        bytes: candidate.bytes,
        rule_id: candidate.rule_id.clone(),
        category: candidate.category,
        safety: candidate.safety,
        requires_sudo: candidate.requires_sudo,
        risk_score: candidate.risk_score,
    }
}

fn default_free_plan_path() -> PathBuf {
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    PathBuf::from(format!("rclean-free-{stamp}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ActivityInfo, Category, ProjectReport, Summary};

    fn candidate(name: &str, bytes: u64, safety: Safety, staleness: Option<u64>) -> Candidate {
        Candidate {
            path: format!("/tmp/proj/{name}"),
            name: name.to_string(),
            rule_id: "rust.target".to_string(),
            category: Category::Build,
            bytes,
            safety,
            requires_sudo: false,
            reasons: vec!["test".to_string()],
            warnings: Vec::new(),
            restore_hint: "cargo build".to_string(),
            risk_score: 0.1,
            staleness_days: staleness,
        }
    }

    fn report_with(candidates: Vec<Candidate>) -> ScanReport {
        ScanReport {
            schema_version: 1,
            tool_version: "test".to_string(),
            scanned_at: "2026-07-03T00:00:00Z".to_string(),
            roots: vec!["/tmp".to_string()],
            disk_attribution: None,
            warnings: Vec::new(),
            stale_after_days: 30,
            summary: Summary {
                projects_scanned: 1,
                projects_with_candidates: 1,
                candidates: candidates.len(),
                safe_candidates: candidates.len(),
                caution_candidates: 0,
                blocked_candidates: 0,
                report_only_candidates: 0,
                total_bytes: candidates.iter().map(|c| c.bytes).sum(),
            },
            projects: vec![ProjectReport {
                path: "/tmp/proj".to_string(),
                kind: "Rust".to_string(),
                markers: vec!["Cargo.toml".to_string()],
                git: None,
                activity: ActivityInfo {
                    last_modified: "2026-05-01T00:00:00Z".to_string(),
                    source: "test".to_string(),
                },
                total_bytes: candidates.iter().map(|c| c.bytes).sum(),
                project_bytes: 100,
                artifact_percent: 50.0,
                candidates,
            }],
        }
    }

    #[test]
    fn prefers_stale_candidates_over_larger_fresh_ones() {
        let report = report_with(vec![
            candidate("fresh-large", 3_000, Safety::Safe, Some(0)),
            candidate("stale-small", 2_000, Safety::Safe, Some(90)),
        ]);
        let proposal = select_for_target(&report, 1_500);
        assert_eq!(proposal.candidates.len(), 1);
        assert_eq!(proposal.candidates[0].candidate.name, "stale-small");
    }

    #[test]
    fn never_selects_non_safe_candidates_even_when_target_unmet() {
        let report = report_with(vec![
            candidate("safe-small", 1_000, Safety::Safe, Some(10)),
            candidate("caution-huge", 100_000, Safety::Caution, Some(90)),
            candidate("blocked-huge", 100_000, Safety::Blocked, Some(90)),
            candidate("report-only-huge", 100_000, Safety::ReportOnly, Some(90)),
        ]);
        let proposal = select_for_target(&report, 50_000);
        assert_eq!(proposal.candidates.len(), 1);
        assert_eq!(proposal.candidates[0].candidate.name, "safe-small");
        assert!(proposal.total_bytes < 50_000);
    }

    #[test]
    fn prunes_picks_the_target_can_spare() {
        let report = report_with(vec![
            candidate("oldest", 1_000, Safety::Safe, Some(90)),
            candidate("older", 4_000, Safety::Safe, Some(60)),
            candidate("old", 2_000, Safety::Safe, Some(40)),
        ]);
        // Greedy picks oldest(1000) + older(4000) = 5000 >= 4500, then
        // the prune drops nothing (5000 - 1000 < 4500, 5000 - 4000 < 4500).
        let proposal = select_for_target(&report, 4_500);
        assert_eq!(proposal.total_bytes, 5_000);
        // A smaller target lets the prune drop the low-ranked pick.
        let proposal = select_for_target(&report, 900);
        assert_eq!(proposal.candidates.len(), 1);
        assert_eq!(proposal.candidates[0].candidate.name, "oldest");
        assert_eq!(proposal.total_bytes, 1_000);
    }
}
