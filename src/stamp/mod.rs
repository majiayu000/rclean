mod sweep;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::clean::SelectedCandidate;
use crate::cli::StampArgs;
use crate::error::RcleanError;
use crate::model::{Candidate, Safety, ScanReport};
use crate::{clean, scan};

const STAMP_FILE: &str = ".rclean-stamp";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StampCommandReport {
    stamped: usize,
    swept: usize,
    plan: Option<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct StampRecord {
    pub stamp_id: String,
    pub timestamp: String,
}

pub fn run(args: StampArgs) -> Result<ExitCode, RcleanError> {
    let roots = args.common.paths_or_current_dir();
    clean::check_broad_roots(&roots)?;

    let options = args.common.to_scan_options()?;
    let report = scan::scan(&roots, &options)?;

    if args.sweep {
        return sweep::run(args, report);
    }

    let mut warnings = Vec::new();
    let stamped = write_stamps(&report, &mut warnings)?;
    let command_report = StampCommandReport {
        stamped,
        swept: 0,
        plan: None,
        warnings,
    };
    print_stamp_report(&command_report, args.common.json)?;

    if stamped == 0 {
        Ok(ExitCode::from(3))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn write_stamps(report: &ScanReport, warnings: &mut Vec<String>) -> Result<usize, RcleanError> {
    let mut stamped = 0;
    for candidate in stampable_candidates(report) {
        let path = PathBuf::from(&candidate.path);
        match write_stamp(&path) {
            Ok(()) => stamped += 1,
            Err(err) => warnings.push(format!("skipped {}: {err}", path.display())),
        }
    }
    Ok(stamped)
}

fn write_stamp(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| format!("cannot read candidate metadata: {err}"))?;
    if metadata.file_type().is_symlink() {
        return Err("candidate is a symlink".to_string());
    }
    if !metadata.is_dir() {
        return Err("candidate is not a directory".to_string());
    }

    let record = StampRecord {
        stamp_id: new_stamp_id(),
        timestamp: Utc::now().to_rfc3339(),
    };
    let json = serde_json::to_vec_pretty(&record).map_err(|err| err.to_string())?;
    fs::write(path.join(STAMP_FILE), json).map_err(|err| format!("cannot write stamp: {err}"))
}

pub(super) fn stampable_candidates(report: &ScanReport) -> impl Iterator<Item = &Candidate> {
    report
        .projects
        .iter()
        .flat_map(|project| project.candidates.iter())
        .filter(|candidate| {
            matches!(candidate.safety, Safety::Safe | Safety::Caution) && !candidate.requires_sudo
        })
}

pub(super) fn to_selected(candidate: &Candidate) -> SelectedCandidate {
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

pub(super) fn stamp_path(candidate_path: &Path) -> PathBuf {
    candidate_path.join(STAMP_FILE)
}

fn print_stamp_report(report: &StampCommandReport, json: bool) -> Result<(), RcleanError> {
    if json {
        let json = serde_json::to_string_pretty(report)?;
        println!("{json}");
    } else {
        println!("Stamped: {}", report.stamped);
        if report.swept > 0 {
            println!("Sweep candidates: {}", report.swept);
        }
        if let Some(plan) = &report.plan {
            eprintln!("wrote action plan: {plan}");
        }
        for warning in &report.warnings {
            eprintln!("warning: {warning}");
        }
    }
    Ok(())
}

fn new_stamp_id() -> String {
    let now = Utc::now();
    format!("stamp-{}-{}", now.timestamp_millis(), std::process::id())
}
