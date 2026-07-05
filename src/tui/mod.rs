mod search;
mod select;
mod theme;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrono::Utc;

use crate::clean::{SelectedCandidate, select_interactively_text};
use crate::cli::CommonScanArgs;
use crate::error::RcleanError;
use crate::model::ScanReport;
use crate::{plan, scan};

pub fn select_candidates(
    report: &ScanReport,
    include_caution: bool,
) -> Result<Vec<SelectedCandidate>, crate::error::CleanError> {
    if !theme::supports_alternate_screen() {
        eprintln!("alternate screen unavailable; falling back to text selection");
        return select_interactively_text(report, include_caution);
    }
    select::run(report)
}

pub fn select_candidates_with_preselected(
    report: &ScanReport,
    include_caution: bool,
    preselected_paths: &std::collections::BTreeSet<std::path::PathBuf>,
) -> Result<Vec<SelectedCandidate>, crate::error::CleanError> {
    if !theme::supports_alternate_screen() {
        eprintln!("alternate screen unavailable; falling back to text selection");
        return crate::clean::select_interactively_text_with_preselected(
            report,
            include_caution,
            preselected_paths,
        );
    }
    select::run_with_preselected(report, preselected_paths)
}

pub fn run_command(args: CommonScanArgs) -> Result<ExitCode, RcleanError> {
    let options = args.to_scan_options()?;
    let report = scan::scan(&args.paths_or_current_dir(), &options)?;
    let selected = select_candidates(&report, args.include_caution)?;
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
