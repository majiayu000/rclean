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
    let mut selected = Vec::new();
    for project in &report.projects {
        for candidate in &project.candidates {
            if candidate.safety == Safety::Blocked {
                continue;
            }
            if args.all {
                if candidate.safety == Safety::Safe
                    || (candidate.safety == Safety::Caution && args.common.include_caution)
                {
                    selected.push(to_selected(candidate));
                }
            } else if ask_candidate(candidate)? {
                selected.push(to_selected(candidate));
            }
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

fn ask_candidate(candidate: &Candidate) -> Result<bool, String> {
    if candidate.safety == Safety::Blocked {
        return Ok(false);
    }
    print!(
        "Clean {} ({}, {}, {})? [y/N] ",
        candidate.path, candidate.rule_id, candidate.safety, candidate.bytes
    );
    io::stdout()
        .flush()
        .map_err(|err| format!("failed to flush stdout: {err}"))?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| format!("failed to read answer: {err}"))?;
    let answer = input.trim().to_ascii_lowercase();
    Ok(answer == "y" || answer == "yes")
}

fn to_selected(candidate: &Candidate) -> SelectedCandidate {
    SelectedCandidate {
        path: PathBuf::from(&candidate.path),
        bytes: candidate.bytes,
        rule_id: candidate.rule_id.clone(),
    }
}
