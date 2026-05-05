use std::io::{self, Write};
use std::path::PathBuf;

use crate::cli::CleanArgs;
use crate::model::{Candidate, Safety, ScanReport, format_bytes};

#[derive(Debug, Clone)]
pub struct SelectedCandidate {
    pub path: PathBuf,
    pub bytes: u64,
    pub rule_id: String,
}

#[derive(Debug, Default)]
pub struct CleanResult {
    pub cleaned: Vec<SelectedCandidate>,
    pub failed: Vec<(SelectedCandidate, String)>,
}

pub fn select_candidates(
    report: &ScanReport,
    args: &CleanArgs,
) -> Result<Vec<SelectedCandidate>, String> {
    let candidates = selectable_candidates(report);

    if !args.all {
        return select_interactively(&candidates, args.common.include_caution);
    }

    let mut selected = Vec::new();
    for candidate in candidates {
        if candidate.safety == Safety::Safe
            || (candidate.safety == Safety::Caution && args.common.include_caution)
        {
            selected.push(to_selected(candidate));
        }
    }
    Ok(selected)
}

fn selectable_candidates(report: &ScanReport) -> Vec<&Candidate> {
    let mut candidates = Vec::new();
    for project in &report.projects {
        for candidate in &project.candidates {
            if candidate.safety != Safety::Blocked {
                candidates.push(candidate);
            }
        }
    }
    candidates
}

fn select_interactively(
    candidates: &[&Candidate],
    include_caution: bool,
) -> Result<Vec<SelectedCandidate>, String> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    println!();
    println!("Select candidates to clean:");
    for (index, candidate) in candidates.iter().enumerate() {
        println!(
            "  {:>2}. {:<8} {:<8} {:>10} {} ({})",
            index + 1,
            candidate.safety,
            candidate.category,
            format_bytes(candidate.bytes),
            candidate.path,
            candidate.rule_id
        );
    }
    println!("Enter numbers/ranges like 1,3,5 or 2-4. Use 'a' for all safe. Empty selects none.");
    print!("Selection: ");
    io::stdout()
        .flush()
        .map_err(|err| format!("failed to flush stdout: {err}"))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| format!("failed to read selection: {err}"))?;

    let selected_indices = parse_selection(input.trim(), candidates.len())?;
    let mut selected = Vec::new();
    for index in selected_indices {
        let candidate = candidates[index];
        if candidate.safety == Safety::Safe
            || (candidate.safety == Safety::Caution && include_caution)
        {
            selected.push(to_selected(candidate));
        }
    }
    Ok(selected)
}

pub fn print_plan(selected: &[SelectedCandidate], permanent: bool, dry_run: bool) {
    if selected.is_empty() {
        println!();
        println!("Nothing selected.");
        return;
    }

    let total: u64 = selected.iter().map(|candidate| candidate.bytes).sum();
    println!();
    println!(
        "Plan: {} candidates, {} selected, mode: {}{}",
        selected.len(),
        format_bytes(total),
        if permanent { "permanent" } else { "trash" },
        if dry_run { " (dry run)" } else { "" }
    );
    for candidate in selected {
        println!(
            "  - {} ({}, {})",
            candidate.path.display(),
            candidate.rule_id,
            format_bytes(candidate.bytes)
        );
    }
}

pub fn confirm_if_needed(selected: &[SelectedCandidate], args: &CleanArgs) -> Result<(), String> {
    if args.yes {
        return Ok(());
    }

    let total: u64 = selected.iter().map(|candidate| candidate.bytes).sum();
    let mode = if args.permanent {
        "permanently delete"
    } else {
        "move to Trash"
    };
    print!(
        "Confirm: {mode} {} candidates ({})? [y/N] ",
        selected.len(),
        format_bytes(total)
    );
    io::stdout()
        .flush()
        .map_err(|err| format!("failed to flush stdout: {err}"))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| format!("failed to read confirmation: {err}"))?;
    let answer = input.trim().to_ascii_lowercase();
    if answer == "y" || answer == "yes" {
        Ok(())
    } else {
        Err("clean cancelled".to_string())
    }
}

pub fn delete_selected(
    selected: &[SelectedCandidate],
    permanent: bool,
) -> Result<CleanResult, String> {
    let mut result = CleanResult::default();

    for candidate in selected {
        let outcome = if permanent {
            std::fs::remove_dir_all(&candidate.path).map_err(|err| err.to_string())
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

pub fn print_clean_result(result: &CleanResult) {
    let total: u64 = result.cleaned.iter().map(|candidate| candidate.bytes).sum();
    println!();
    println!(
        "Cleaned: {} candidates, {}",
        result.cleaned.len(),
        format_bytes(total)
    );
    if !result.failed.is_empty() {
        println!("Failed: {}", result.failed.len());
        for (candidate, error) in &result.failed {
            println!("  - {}: {}", candidate.path.display(), error);
        }
    }
}

pub fn parse_selection(input: &str, count: usize) -> Result<Vec<usize>, String> {
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
                return Err(format!("invalid range '{part}'"));
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

fn parse_selection_number(raw: &str, count: usize) -> Result<usize, String> {
    let number = raw
        .trim()
        .parse::<usize>()
        .map_err(|_| format!("invalid selection '{raw}'"))?;
    if number == 0 || number > count {
        return Err(format!("selection {number} is out of range 1-{count}"));
    }
    Ok(number - 1)
}

fn to_selected(candidate: &Candidate) -> SelectedCandidate {
    SelectedCandidate {
        path: PathBuf::from(&candidate.path),
        bytes: candidate.bytes,
        rule_id: candidate.rule_id.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_interactive_selection() {
        assert_eq!(parse_selection("", 5).unwrap(), Vec::<usize>::new());
        assert_eq!(parse_selection("a", 3).unwrap(), vec![0, 1, 2]);
        assert_eq!(parse_selection("1,3-4,3", 5).unwrap(), vec![0, 2, 3]);
        assert!(parse_selection("0", 3).is_err());
        assert!(parse_selection("4", 3).is_err());
        assert!(parse_selection("3-1", 3).is_err());
    }
}
