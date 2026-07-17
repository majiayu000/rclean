//! `rclean free <size>` — goal-oriented cleanup (spec:
//! docs/specs/v0.2-best-ux.md §3.3 C1).
//!
//! Proposes the smallest set of `safe` candidates whose total meets the
//! requested reclaim target, preferring stale projects, and writes the
//! proposal as a reviewable ActionPlan unless `--interactive` is used.
//! It never pre-selects caution, blocked, report-only, or sudo candidates.

use std::collections::BTreeSet;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrono::Utc;
use serde::Serialize;

use crate::clean::{CleanResult, SelectedCandidate};
use crate::cli::FreeArgs;
use crate::error::{CleanError, RcleanError};
use crate::model::{Candidate, Safety, ScanReport, format_bytes};
use crate::stdio::{self, outln};
use crate::{parse, plan, scan};

pub fn run(args: FreeArgs) -> Result<ExitCode, RcleanError> {
    if args.interactive {
        if args.common.write_plan.is_some() {
            return Err(CleanError::Generic(
                "free --interactive cannot be combined with --write-plan; omit --interactive to write a plan"
                    .to_string(),
            )
            .into());
        }
        if args.common.json {
            return Err(CleanError::Generic(
                "free --interactive cannot be combined with --json; interactive cleanup is human-readable"
                    .to_string(),
            )
            .into());
        }
        ensure_interactive_terminal()?;
    }
    let target = parse::parse_size(&args.target)?;
    let options = args.common.to_scan_options()?;
    let report = scan::scan(&args.common.paths_or_current_dir(), &options)?;

    let proposal = select_for_target(&report, target);
    let status = if !proposal.candidates.is_empty() && proposal.total_bytes >= target {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(3)
    };

    if proposal.candidates.is_empty() {
        let output_result = if args.common.json {
            print_json_proposal(target, &proposal, None)
        } else {
            print_no_candidates(target)
        };
        return stdio::finish_output(status, output_result);
    }

    if !args.common.json && !stdio::continue_after_output(print_human_proposal(target, &proposal))?
    {
        return Ok(status);
    }

    if args.interactive {
        return run_interactive(args, &report, &proposal);
    }

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

    if args.common.json {
        stdio::finish_output(
            status,
            print_json_proposal(target, &proposal, Some(&plan_path)),
        )
    } else {
        stdio::finish_output(
            status,
            print_human_plan_result(target, &proposal, &plan_path),
        )
    }
}

fn print_no_candidates(target: u64) -> Result<(), RcleanError> {
    outln!(
        "no safe candidates available; cannot free {}",
        format_bytes(target)
    );
    Ok(())
}

fn print_human_proposal(target: u64, proposal: &Proposal<'_>) -> Result<(), RcleanError> {
    outln!(
        "Proposed set to free {} (smallest safe set, stale projects first):",
        format_bytes(target)
    );
    for entry in &proposal.candidates {
        outln!(
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
    outln!(
        "Total: {} of {} requested",
        format_bytes(proposal.total_bytes),
        format_bytes(target)
    );
    Ok(())
}

fn print_human_plan_result(
    target: u64,
    proposal: &Proposal<'_>,
    plan_path: &Path,
) -> Result<(), RcleanError> {
    outln!("wrote action plan: {}", plan_path.display());
    outln!(
        "review it, then run: rclean clean --plan {}",
        plan_path.display()
    );
    if proposal.total_bytes < target {
        // Never widen the selection silently (U-29): the gap is stated
        // explicitly and the exit code says the target was not met.
        outln!(
            "target not met: safe candidates cover {}, short by {}",
            format_bytes(proposal.total_bytes),
            format_bytes(target - proposal.total_bytes)
        );
    }
    Ok(())
}

fn print_json_proposal(
    target: u64,
    proposal: &Proposal<'_>,
    plan_path: Option<&Path>,
) -> Result<(), RcleanError> {
    let output = FreeProposalOutput {
        schema_version: 1,
        target_bytes: target,
        selected_bytes: proposal.total_bytes,
        target_met: !proposal.candidates.is_empty() && proposal.total_bytes >= target,
        plan_path: plan_path.map(|path| path.display().to_string()),
        candidates: proposal
            .candidates
            .iter()
            .map(|entry| entry.candidate)
            .collect(),
    };
    let json = serde_json::to_string_pretty(&output)?;
    outln!("{json}");
    Ok(())
}

fn ensure_interactive_terminal() -> Result<(), RcleanError> {
    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        return Ok(());
    }
    Err(CleanError::Generic(
        "free --interactive requires an interactive terminal; no cleanup was run".to_string(),
    )
    .into())
}

fn run_interactive(
    args: FreeArgs,
    report: &ScanReport,
    proposal: &Proposal<'_>,
) -> Result<ExitCode, RcleanError> {
    let preselected_paths: BTreeSet<PathBuf> = proposal
        .candidates
        .iter()
        .map(|entry| PathBuf::from(&entry.candidate.path))
        .collect();
    let selected = select_interactively(report, args.common.include_caution, &preselected_paths)?;
    let delete_mode = default_interactive_delete_mode();
    let clean_args = interactive_clean_args(args);

    if !clean_args.common.json {
        crate::clean::print_plan(&selected, delete_mode, false)?;
    }
    if selected.is_empty() {
        return Ok(ExitCode::from(3));
    }

    crate::clean::confirm_if_needed(&selected, &clean_args)?;
    let result = delete_interactive_selected(&selected, &clean_args)?;
    let status = if result.failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    };
    let output_result = crate::clean::print_clean_result(&result).and_then(|()| {
        if clean_args.common.json {
            Ok(())
        } else {
            crate::clean::print_recovery_summary(&result, delete_mode)
        }
    });
    stdio::finish_output(status, output_result)
}

fn select_interactively(
    report: &ScanReport,
    include_caution: bool,
    preselected_paths: &BTreeSet<PathBuf>,
) -> Result<Vec<SelectedCandidate>, RcleanError> {
    #[cfg(feature = "tui")]
    {
        crate::tui::select_candidates_with_preselected(report, include_caution, preselected_paths)
            .map_err(Into::into)
    }
    #[cfg(not(feature = "tui"))]
    {
        crate::clean::select_interactively_text_with_preselected(
            report,
            include_caution,
            preselected_paths,
        )
        .map_err(Into::into)
    }
}

fn interactive_clean_args(args: FreeArgs) -> crate::cli::CleanArgs {
    crate::cli::CleanArgs {
        common: args.common,
        all: false,
        dry_run: false,
        permanent: false,
        #[cfg(feature = "graveyard")]
        graveyard: true,
        yes: false,
        plan: None,
        audit_log: None,
        tui: false,
        allow_broad_root: false,
    }
}

fn default_interactive_delete_mode() -> &'static str {
    #[cfg(feature = "graveyard")]
    {
        "graveyard"
    }
    #[cfg(not(feature = "graveyard"))]
    {
        "trash"
    }
}

fn delete_interactive_selected(
    selected: &[SelectedCandidate],
    args: &crate::cli::CleanArgs,
) -> Result<CleanResult, RcleanError> {
    #[cfg(feature = "graveyard")]
    {
        if args.graveyard {
            let yard = crate::graveyard::Graveyard::open(crate::graveyard::default_root());
            return crate::clean::delete_selected_into_graveyard(selected, &yard, None)
                .map_err(Into::into);
        }
        crate::clean::delete_selected(selected, args.permanent, None).map_err(Into::into)
    }
    #[cfg(not(feature = "graveyard"))]
    {
        crate::clean::delete_selected(selected, args.permanent, None).map_err(Into::into)
    }
}

struct RankedCandidate<'a> {
    candidate: &'a Candidate,
}

struct Proposal<'a> {
    candidates: Vec<RankedCandidate<'a>>,
    total_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FreeProposalOutput<'a> {
    schema_version: u32,
    target_bytes: u64,
    selected_bytes: u64,
    target_met: bool,
    plan_path: Option<String>,
    candidates: Vec<&'a Candidate>,
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
mod tests;
