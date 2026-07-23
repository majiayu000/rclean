use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard};

use crate::model::{ActivityInfo, Candidate, Category, ProjectReport, Safety, ScanReport, Summary};

/// Serializes every unit test that mutates the process environment.
///
/// `cargo test --lib` runs tests in one process, in parallel. Home and
/// XDG variables are process-global, so two modules overriding them
/// concurrently produce flaky, order-dependent failures. This lock is
/// crate-wide on purpose: a per-module mutex would not serialize
/// against another module touching the same variable.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Swaps environment variables for the duration of a test and restores
/// the previous values on drop.
pub(crate) fn with_env_vars(vars: &[(&str, Option<&str>)]) -> EnvGuard {
    let lock = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
    let previous = vars
        .iter()
        .map(|(key, _)| ((*key).to_string(), std::env::var_os(key)))
        .collect();
    for (key, value) in vars {
        // SAFETY: ENV_LOCK serializes every test that mutates the
        // process environment, and Drop restores the previous values.
        unsafe {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
    EnvGuard {
        previous,
        _lock: lock,
    }
}

pub(crate) struct EnvGuard {
    previous: Vec<(String, Option<OsString>)>,
    _lock: MutexGuard<'static, ()>,
}

impl EnvGuard {
    /// Re-points variables while still holding the lock, so one test
    /// can walk through several environments without releasing it.
    pub(crate) fn set(&self, vars: &[(&str, Option<&str>)]) {
        for (key, value) in vars {
            // SAFETY: see with_env_vars; the lock is still held.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            // SAFETY: see with_env_vars.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}

pub(crate) fn ranking_candidate(
    name: &str,
    bytes: u64,
    safety: Safety,
    staleness_days: Option<u64>,
) -> Candidate {
    Candidate {
        path: format!("/tmp/proj/{name}"),
        name: name.to_string(),
        rule_id: "rust.target".to_string(),
        category: Category::Build,
        bytes,
        safety,
        requires_sudo: false,
        reasons: vec!["test".to_string()],
        warnings: Vec::new(),
        restore_hint: "cargo build".to_string(),
        risk_score: 0.1,
        staleness_days,
    }
}

pub(crate) fn ranking_report(candidates: Vec<Candidate>) -> ScanReport {
    ScanReport {
        schema_version: 1,
        tool_version: "test".to_string(),
        scanned_at: "2026-07-03T00:00:00Z".to_string(),
        roots: vec!["/tmp".to_string()],
        disk_attribution: None,
        warnings: Vec::new(),
        stale_after_days: 30,
        summary: Summary {
            projects_scanned: 1,
            projects_with_candidates: 1,
            candidates: candidates.len(),
            safe_candidates: candidates.len(),
            caution_candidates: 0,
            blocked_candidates: 0,
            report_only_candidates: 0,
            total_bytes: candidates.iter().map(|c| c.bytes).sum(),
        },
        projects: vec![ProjectReport {
            path: "/tmp/proj".to_string(),
            kind: "Rust".to_string(),
            markers: vec!["Cargo.toml".to_string()],
            git: None,
            activity: ActivityInfo {
                last_modified: "2026-05-01T00:00:00Z".to_string(),
                source: "test".to_string(),
            },
            total_bytes: candidates.iter().map(|c| c.bytes).sum(),
            project_bytes: 100,
            artifact_percent: 50.0,
            candidates,
        }],
    }
}
