mod search;
mod select;
mod theme;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrono::Utc;

use crate::clean::{SelectedCandidate, SelectionOutcome, select_interactively_text};
use crate::cli::CommonScanArgs;
use crate::error::RcleanError;
use crate::model::ScanReport;
use crate::{plan, scan};
use select::SelectionNextStep;

pub fn select_candidates(
    report: &ScanReport,
    include_caution: bool,
    dry_run: bool,
    skip_confirmation: bool,
) -> Result<SelectionOutcome, crate::error::CleanError> {
    if !theme::supports_alternate_screen() {
        eprintln!("alternate screen unavailable; falling back to text selection");
        return select_interactively_text(report, include_caution).map(SelectionOutcome::Confirmed);
    }
    select::run(
        report,
        SelectionNextStep::for_clean(dry_run, skip_confirmation),
    )
}

pub fn select_candidates_with_preselected(
    report: &ScanReport,
    include_caution: bool,
    preselected_paths: &std::collections::BTreeSet<std::path::PathBuf>,
) -> Result<SelectionOutcome, crate::error::CleanError> {
    if !theme::supports_alternate_screen() {
        eprintln!("alternate screen unavailable; falling back to text selection");
        return crate::clean::select_interactively_text_with_preselected(
            report,
            include_caution,
            preselected_paths,
        )
        .map(SelectionOutcome::Confirmed);
    }
    select::run_with_preselected(report, preselected_paths, SelectionNextStep::ConfirmCleanup)
}

fn select_candidates_for_plan(
    report: &ScanReport,
    include_caution: bool,
) -> Result<SelectionOutcome, crate::error::CleanError> {
    if !theme::supports_alternate_screen() {
        eprintln!("alternate screen unavailable; falling back to text selection");
        return select_interactively_text(report, include_caution).map(SelectionOutcome::Confirmed);
    }
    select::run(report, SelectionNextStep::WriteActionPlan)
}

pub fn run_command(args: CommonScanArgs) -> Result<ExitCode, RcleanError> {
    let options = args.to_scan_options()?;
    let report = scan::scan(&args.paths_or_current_dir(), &options)?;
    let selected = match select_candidates_for_plan(&report, args.include_caution)? {
        SelectionOutcome::Confirmed(selected) => selected,
        SelectionOutcome::Cancelled => return Ok(ExitCode::from(3)),
    };
    if selected.is_empty() {
        eprintln!("no candidates selected");
        return Ok(ExitCode::from(3));
    }

    let plan_path = args
        .write_plan
        .clone()
        .unwrap_or_else(default_tui_plan_path);
    write_plan(&report, &selected, &plan_path)?;
    println!("wrote action plan: {}", plan_path.display());
    Ok(ExitCode::SUCCESS)
}

fn write_plan(
    report: &ScanReport,
    selected: &[SelectedCandidate],
    path: &Path,
) -> Result<(), RcleanError> {
    plan::write_selected_action_plan(report, path, selected, "trash")?;
    Ok(())
}

fn default_tui_plan_path() -> PathBuf {
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    PathBuf::from(format!("rclean-tui-{stamp}.json"))
}
