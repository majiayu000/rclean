use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use walkdir::WalkDir;

use crate::cli::StampArgs;
use crate::error::RcleanError;
use crate::model::{Candidate, ScanReport};
use crate::plan;

use super::{
    StampCommandReport, StampRecord, print_stamp_report, stamp_path, stampable_candidates,
    to_selected,
};

pub(super) fn run(args: StampArgs, report: ScanReport) -> Result<ExitCode, RcleanError> {
    let Some(plan_path) = args.common.write_plan.as_deref() else {
        return Err(RcleanError::from(
            "stamp --sweep requires --write-plan <path>".to_string(),
        ));
    };

    let mut warnings = Vec::new();
    let selected = select_unchanged_stamped_candidates(&report, &mut warnings);
    plan::write_selected_action_plan(&report, plan_path, &selected, "trash")?;

    let command_report = StampCommandReport {
        stamped: 0,
        swept: selected.len(),
        plan: Some(plan_path.display().to_string()),
        warnings,
    };
    print_stamp_report(&command_report, args.common.json)?;

    if selected.is_empty() {
        Ok(ExitCode::from(3))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn select_unchanged_stamped_candidates(
    report: &ScanReport,
    warnings: &mut Vec<String>,
) -> Vec<crate::clean::SelectedCandidate> {
    stampable_candidates(report)
        .filter_map(
            |candidate| match candidate_is_unchanged_since_stamp(candidate, warnings) {
                Some(true) => Some(to_selected(candidate)),
                Some(false) | None => None,
            },
        )
        .collect()
}

fn candidate_is_unchanged_since_stamp(
    candidate: &Candidate,
    warnings: &mut Vec<String>,
) -> Option<bool> {
    let candidate_path = Path::new(&candidate.path);
    let stamp_path = stamp_path(candidate_path);
    if !stamp_path.exists() {
        return None;
    }

    let record = match read_stamp(&stamp_path) {
        Ok(record) => record,
        Err(err) => {
            warnings.push(format!("skipped {}: {err}", candidate.path));
            return None;
        }
    };
    let stamp_time = match DateTime::parse_from_rfc3339(&record.timestamp) {
        Ok(timestamp) => SystemTime::from(timestamp.with_timezone(&Utc)),
        Err(err) => {
            warnings.push(format!(
                "skipped {}: invalid stamp timestamp: {err}",
                candidate.path
            ));
            return None;
        }
    };

    match latest_modified_excluding_stamp(candidate_path, &stamp_path) {
        Ok(Some(modified)) => Some(modified <= stamp_time),
        Ok(None) => Some(true),
        Err(err) => {
            warnings.push(format!("skipped {}: {err}", candidate.path));
            None
        }
    }
}

fn read_stamp(path: &Path) -> Result<StampRecord, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("cannot read stamp: {err}"))?;
    serde_json::from_str(&raw).map_err(|err| format!("invalid stamp json: {err}"))
}

fn latest_modified_excluding_stamp(
    candidate_path: &Path,
    stamp_path: &Path,
) -> Result<Option<SystemTime>, String> {
    let mut latest = None;
    for entry in WalkDir::new(candidate_path).follow_links(false) {
        let entry = entry.map_err(|err| format!("cannot walk candidate: {err}"))?;
        let path = entry.path();
        if path == candidate_path || path == stamp_path {
            continue;
        }
        let metadata = fs::symlink_metadata(path)
            .map_err(|err| format!("cannot read metadata for {}: {err}", path.display()))?;
        let modified = metadata
            .modified()
            .map_err(|err| format!("cannot read mtime for {}: {err}", path.display()))?;
        latest = Some(latest.map_or(modified, |current: SystemTime| current.max(modified)));
    }
    Ok(latest)
}
