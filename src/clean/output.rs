use std::io::{self, Write};

use crate::cli::CleanArgs;
use crate::error::CleanError;
use crate::model::format_bytes;

use super::types::{CleanResult, SelectedCandidate};

pub fn print_plan(selected: &[SelectedCandidate], delete_mode: &str, dry_run: bool) {
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
        delete_mode,
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

pub fn confirm_if_needed(
    selected: &[SelectedCandidate],
    args: &CleanArgs,
) -> Result<(), CleanError> {
    if args.yes {
        return Ok(());
    }

    let total: u64 = selected.iter().map(|candidate| candidate.bytes).sum();
    let mode = if args.permanent {
        "permanently delete"
    } else {
        #[cfg(feature = "graveyard")]
        {
            if args.graveyard {
                "move to the rclean graveyard"
            } else {
                "move to Trash"
            }
        }
        #[cfg(not(feature = "graveyard"))]
        {
            "move to Trash"
        }
    };
    print!(
        "Confirm: {mode} {} candidates ({})? [y/N] ",
        selected.len(),
        format_bytes(total)
    );
    io::stdout()
        .flush()
        .map_err(|err| CleanError::Generic(format!("failed to flush stdout: {err}")))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| CleanError::Generic(format!("failed to read confirmation: {err}")))?;
    let answer = input.trim().to_ascii_lowercase();
    if answer == "y" || answer == "yes" {
        Ok(())
    } else {
        Err(CleanError::Generic("clean cancelled".to_string()))
    }
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
