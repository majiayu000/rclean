use std::io::{self, Write};
use std::path::PathBuf;

use crate::cli::CleanArgs;
use crate::error::CleanError;
use crate::model::{Candidate, Safety, ScanReport, format_bytes};

use super::types::{SelectableCandidate, SelectedCandidate};

pub fn select_candidates(
    report: &ScanReport,
    args: &CleanArgs,
) -> Result<Vec<SelectedCandidate>, CleanError> {
    if args.tui {
        #[cfg(feature = "tui")]
        {
            return crate::tui::select_candidates(report, args.common.include_caution);
        }
        #[cfg(not(feature = "tui"))]
        {
            return Err(CleanError::Generic(
                "TUI support is not enabled in this build; rebuild with --features tui".to_string(),
            ));
        }
    }

    select_candidates_text(report, args)
}

fn select_candidates_text(
    report: &ScanReport,
    args: &CleanArgs,
) -> Result<Vec<SelectedCandidate>, CleanError> {
    let candidates = selectable_candidates(report);

    if !args.all {
        return select_interactively(&candidates, args.common.include_caution);
    }

    let mut selected = Vec::new();
    for item in candidates {
        if item.candidate.safety == Safety::Safe
            || (item.candidate.safety == Safety::Caution && args.common.include_caution)
        {
            selected.push(to_selected(item.candidate));
        }
    }
    Ok(selected)
}

#[cfg_attr(not(feature = "tui"), allow(dead_code))]
pub fn select_interactively_text(
    report: &ScanReport,
    include_caution: bool,
) -> Result<Vec<SelectedCandidate>, CleanError> {
    let candidates = selectable_candidates(report);
    select_interactively(&candidates, include_caution)
}

fn selectable_candidates(report: &ScanReport) -> Vec<SelectableCandidate<'_>> {
    let mut candidates = Vec::new();
    for project in &report.projects {
        for candidate in &project.candidates {
            // ReportOnly is excluded from selectable candidates at the
            // same level as Blocked: never offered for cleanup even
            // with `--include-blocked`.
            if candidate.safety != Safety::Blocked
                && candidate.safety != Safety::ReportOnly
                && !candidate.requires_sudo
            {
                candidates.push(SelectableCandidate {
                    project_path: &project.path,
                    candidate,
                });
            }
        }
    }
    candidates
}

fn select_interactively(
    candidates: &[SelectableCandidate<'_>],
    include_caution: bool,
) -> Result<Vec<SelectedCandidate>, CleanError> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    println!();
    println!("Select candidates to clean:");
    let mut current_project = "";
    for (index, item) in candidates.iter().enumerate() {
        let candidate = item.candidate;
        if item.project_path != current_project {
            current_project = item.project_path;
            println!();
            println!("Project: {current_project}");
        }
        let reason = candidate
            .reasons
            .first()
            .or_else(|| candidate.warnings.first())
            .map(String::as_str)
            .unwrap_or("-");
        println!(
            "  {:>2}. {:<8} {:<8} {:>10} {:<24} {}",
            index + 1,
            candidate.safety,
            candidate.category,
            format_bytes(candidate.bytes),
            candidate.name,
            reason
        );
    }
    println!("Enter numbers/ranges like 1,3,5 or 2-4. Use 'a' for all safe. Empty selects none.");
    print!("Selection: ");
    io::stdout()
        .flush()
        .map_err(|err| CleanError::Generic(format!("failed to flush stdout: {err}")))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| CleanError::Generic(format!("failed to read selection: {err}")))?;

    let input = input.trim();
    if input.eq_ignore_ascii_case("a") {
        return Ok(candidates
            .iter()
            .filter(|item| item.candidate.safety == Safety::Safe)
            .map(|item| to_selected(item.candidate))
            .collect());
    }

    let selected_indices = parse_selection(input, candidates.len())?;
    let mut selected = Vec::new();
    for index in selected_indices {
        let candidate = candidates[index].candidate;
        if candidate.safety == Safety::Safe
            || (candidate.safety == Safety::Caution && include_caution)
        {
            selected.push(to_selected(candidate));
        }
    }
    Ok(selected)
}

pub(super) fn parse_selection(input: &str, count: usize) -> Result<Vec<usize>, CleanError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.eq_ignore_ascii_case("a") {
        return Ok((0..count).collect());
    }

    let mut selected = Vec::new();
    for part in trimmed
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if let Some((start, end)) = part.split_once('-') {
            let start = parse_selection_number(start, count)?;
            let end = parse_selection_number(end, count)?;
            if start > end {
                return Err(CleanError::Generic(format!("invalid range '{part}'")));
            }
            for index in start..=end {
                if !selected.contains(&index) {
                    selected.push(index);
                }
            }
        } else {
            let index = parse_selection_number(part, count)?;
            if !selected.contains(&index) {
                selected.push(index);
            }
        }
    }
    Ok(selected)
}

fn parse_selection_number(raw: &str, count: usize) -> Result<usize, CleanError> {
    let number = raw
        .trim()
        .parse::<usize>()
        .map_err(|_| CleanError::Generic(format!("invalid selection '{raw}'")))?;
    if number == 0 || number > count {
        return Err(CleanError::Generic(format!(
            "selection {number} is out of range 1-{count}"
        )));
    }
    Ok(number - 1)
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
