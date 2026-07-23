//! `rclean doctor` — diagnostic for which global-cache rules are
//! applicable on this machine.
//!
//! Each Phase 1 global-path rule has a canonical anchor directory
//! (`~/.cargo`, `~/.gradle`, `~/Library/Developer`, ...). Doctor
//! reports per-rule whether that anchor exists, so the user can
//! see at a glance what `rclean scan --home` will actually touch.
//!
//! No filesystem writes, no subprocess spawns. Pure dir-exists
//! checks. Safe to run on any machine, including CI.
//!
//! See `docs/specs/v0.2-developer-mole.md` §4.3.

mod anchors;
mod common_entries;
mod platform_entries;

use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug)]
pub struct DoctorReport {
    pub entries: Vec<DoctorEntry>,
}

#[derive(Debug)]
pub struct DoctorEntry {
    pub rule_id: &'static str,
    pub anchor: PathBuf,
    pub status: Status,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Applicable,
    Skipped { reason: String },
}

#[derive(Debug, Default)]
pub struct DoctorOptions {
    pub include_docker: bool,
}

impl DoctorReport {
    pub fn applicable_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.status, Status::Applicable))
            .count()
    }

    pub fn total_count(&self) -> usize {
        self.entries.len()
    }
}

pub fn diagnose() -> DoctorReport {
    diagnose_with_options(DoctorOptions::default())
}

pub fn diagnose_with_options(options: DoctorOptions) -> DoctorReport {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return DoctorReport {
            entries: Vec::new(),
        };
    };

    let mut entries = common_entries::collect(&home);
    platform_entries::extend(&mut entries, &home);

    if options.include_docker {
        entries.push(check_docker_daemon(crate::docker::DOCTOR_PROBE_TIMEOUT));
    }

    DoctorReport { entries }
}

fn check_anchor(
    rule_id: &'static str,
    anchor: PathBuf,
    missing_reason: &'static str,
) -> DoctorEntry {
    let exists = anchor.is_dir();
    let status = if exists {
        Status::Applicable
    } else {
        Status::Skipped {
            reason: missing_reason.to_string(),
        }
    };
    DoctorEntry {
        rule_id,
        anchor,
        status,
    }
}

#[cfg(target_os = "macos")]
fn skipped_anchor(rule_id: &'static str, anchor: PathBuf, reason: &'static str) -> DoctorEntry {
    DoctorEntry {
        rule_id,
        anchor,
        status: Status::Skipped {
            reason: reason.to_string(),
        },
    }
}

fn check_any_anchor(
    rule_id: &'static str,
    anchors: Vec<PathBuf>,
    missing_reason: &'static str,
) -> DoctorEntry {
    if let Some(anchor) = anchors.iter().find(|anchor| anchor.is_dir()) {
        return DoctorEntry {
            rule_id,
            anchor: anchor.clone(),
            status: Status::Applicable,
        };
    }

    DoctorEntry {
        rule_id,
        anchor: anchors
            .into_iter()
            .next()
            .unwrap_or_else(|| PathBuf::from("(unknown)")),
        status: Status::Skipped {
            reason: missing_reason.to_string(),
        },
    }
}

fn check_docker_daemon(timeout: Duration) -> DoctorEntry {
    match crate::docker::probe_for_doctor(timeout) {
        crate::docker::DockerDoctorStatus::Available { server_version } => DoctorEntry {
            rule_id: "docker.daemon",
            anchor: server_version
                .map(|version| PathBuf::from(format!("Docker Server {version}")))
                .unwrap_or_else(|| PathBuf::from("Docker daemon")),
            status: Status::Applicable,
        },
        crate::docker::DockerDoctorStatus::Skipped { reason } => DoctorEntry {
            rule_id: "docker.daemon",
            anchor: PathBuf::from("docker"),
            status: Status::Skipped { reason },
        },
    }
}

#[cfg(test)]
mod tests;
