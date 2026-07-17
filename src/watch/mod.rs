use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use chrono::Utc;
use notify::{Event, RecursiveMode, Watcher};

use crate::cli::WatchArgs;
use crate::error::{PlanError, RcleanError, ScanError};
use crate::model::{Safety, ScanReport, format_bytes};
use crate::path_util::path_file_name;
use crate::stdio::outln;
use crate::{output, parse, plan, scan};

const IDLE_DEGRADE_AFTER: Duration = Duration::from_secs(20 * 60);

#[derive(Clone, PartialEq)]
struct CandidateSnapshot {
    bytes: u64,
    safety: Safety,
}

type CandidateMap = BTreeMap<String, CandidateSnapshot>;

#[derive(Default)]
struct WatchState {
    by_project: BTreeMap<String, CandidateMap>,
}

impl WatchState {
    fn replace_report(&mut self, report: &ScanReport) {
        for project in &report.projects {
            self.by_project
                .insert(project.path.clone(), candidates_for(report, &project.path));
        }
    }

    fn update_project(&mut self, root: &Path, report: &ScanReport) -> Result<(), RcleanError> {
        let scope = canonical_key(root);
        let scope_path = Path::new(&scope);
        let current_projects: BTreeSet<&str> = report
            .projects
            .iter()
            .map(|project| project.path.as_str())
            .collect();
        let stale_projects: Vec<String> = self
            .by_project
            .keys()
            .filter(|project| Path::new(project).starts_with(scope_path))
            .filter(|project| !current_projects.contains(project.as_str()))
            .cloned()
            .collect();

        for project in stale_projects {
            if let Some(old) = self.by_project.remove(&project) {
                print_diff(&project, &old, &CandidateMap::new())?;
            }
        }

        for project in &report.projects {
            let new = candidates_for(report, &project.path);
            let old = self.by_project.insert(project.path.clone(), new.clone());
            print_diff(&project.path, &old.unwrap_or_default(), &new)?;
        }
        Ok(())
    }
}

pub fn run(args: WatchArgs) -> Result<ExitCode, RcleanError> {
    match run_inner(args) {
        Err(error) if crate::stdio::is_broken_pipe(&error) => Ok(ExitCode::SUCCESS),
        result => result,
    }
}

fn run_inner(args: WatchArgs) -> Result<ExitCode, RcleanError> {
    let options = args.common.to_scan_options()?;
    let roots = args.common.paths_or_current_dir();
    let every = parse::parse_duration(&args.every)?;

    let report = scan::scan(&roots, &options)?;
    output::print_table(&report)?;
    write_timestamped_plan(&args, &report)?;

    let mut state = WatchState::default();
    state.replace_report(&report);

    let (tx, rx) = mpsc::channel();
    let mut watcher = start_watcher(&roots, tx);
    let mut last_event = Instant::now();

    if watcher.is_none() {
        eprintln!(
            "watch unavailable; polling every {} seconds",
            every.as_secs().max(1)
        );
    }

    loop {
        if watcher.is_some() {
            match rx.recv_timeout(every) {
                Ok(Ok(event)) => {
                    last_event = Instant::now();
                    for project_root in affected_project_roots(&event) {
                        refresh_project(&args, &options, &mut state, &project_root)?;
                    }
                }
                Ok(Err(err)) => {
                    return Err(scan_error(format!("watch event error: {err}")));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if last_event.elapsed() >= IDLE_DEGRADE_AFTER {
                        watcher = None;
                        eprintln!(
                            "no lockfile events for 20 minutes; polling every {} seconds",
                            every.as_secs().max(1)
                        );
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    watcher = None;
                    eprintln!(
                        "watcher disconnected; polling every {} seconds",
                        every.as_secs().max(1)
                    );
                }
            }
        } else {
            std::thread::sleep(every);
            for root in &roots {
                refresh_project(&args, &options, &mut state, root)?;
            }
        }
    }
}

fn start_watcher(
    roots: &[PathBuf],
    tx: mpsc::Sender<notify::Result<Event>>,
) -> Option<notify::RecommendedWatcher> {
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    })
    .map_err(|err| {
        eprintln!("failed to start file watcher: {err}");
        err
    })
    .ok()?;

    for root in roots {
        if let Err(err) = watcher.watch(root, RecursiveMode::Recursive) {
            eprintln!("failed to watch {}: {err}", root.display());
            return None;
        }
    }
    Some(watcher)
}

fn refresh_project(
    args: &WatchArgs,
    options: &scan::ScanOptions,
    state: &mut WatchState,
    project_root: &Path,
) -> Result<(), RcleanError> {
    let report = scan::scan(&[project_root.to_path_buf()], options)?;
    state.update_project(project_root, &report)?;
    write_timestamped_plan(args, &report)?;
    Ok(())
}

fn write_timestamped_plan(args: &WatchArgs, report: &ScanReport) -> Result<(), RcleanError> {
    let Some(base_path) = &args.common.write_plan else {
        return Ok(());
    };
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let path = next_timestamped_path(base_path, &stamp)?;
    plan::write_action_plan(report, &path, args.common.include_caution, false, "trash")?;
    eprintln!("wrote action plan: {}", path.display());
    Ok(())
}

fn next_timestamped_path(path: &Path, stamp: &str) -> Result<PathBuf, PlanError> {
    next_timestamped_path_with(path, stamp, 1, Path::try_exists)
}

fn next_timestamped_path_with<F>(
    path: &Path,
    stamp: &str,
    mut sequence: u64,
    mut try_exists: F,
) -> Result<PathBuf, PlanError>
where
    F: FnMut(&Path) -> std::io::Result<bool>,
{
    loop {
        let candidate = timestamped_path(path, stamp, (sequence > 1).then_some(sequence));
        if !try_exists(&candidate).map_err(|source| PlanError::Io {
            path: candidate.clone(),
            source,
        })? {
            return Ok(candidate);
        }
        sequence = sequence.checked_add(1).ok_or_else(|| {
            PlanError::Generic("watch plan collision sequence exhausted".to_string())
        })?;
    }
}

fn timestamped_path(path: &Path, stamp: &str, sequence: Option<u64>) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("rclean-watch");
    let suffix = sequence.map_or_else(String::new, |value| format!("-{value}"));
    let name = match path.extension().and_then(|value| value.to_str()) {
        Some(ext) => format!("{stem}-{stamp}{suffix}.{ext}"),
        None => format!("{stem}-{stamp}{suffix}"),
    };
    path.with_file_name(name)
}

fn candidates_for(report: &ScanReport, project_path: &str) -> CandidateMap {
    report
        .projects
        .iter()
        .find(|project| project.path == project_path)
        .into_iter()
        .flat_map(|project| project.candidates.iter())
        .map(|candidate| {
            (
                candidate.path.clone(),
                CandidateSnapshot {
                    bytes: candidate.bytes,
                    safety: candidate.safety,
                },
            )
        })
        .collect()
}

fn print_diff(project: &str, old: &CandidateMap, new: &CandidateMap) -> Result<(), RcleanError> {
    for (path, candidate) in new {
        match old.get(path) {
            None => outln!(
                "added: {path} ({}, {})",
                format_bytes(candidate.bytes),
                candidate.safety
            ),
            Some(previous) if previous.bytes != candidate.bytes => outln!(
                "changed: {path} ({} -> {}, {})",
                format_bytes(previous.bytes),
                format_bytes(candidate.bytes),
                candidate.safety
            ),
            _ => {}
        }
    }
    for path in old.keys() {
        if !new.contains_key(path) {
            outln!("removed: {path}");
        }
    }
    if old == new {
        outln!("refreshed: {project} (no candidate changes)");
    }
    Ok(())
}

fn affected_project_roots(event: &Event) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for path in &event.paths {
        if let Some(root) = project_root_for_lockfile(path)
            && !roots.contains(&root)
        {
            roots.push(root);
        }
    }
    roots
}

fn project_root_for_lockfile(path: &Path) -> Option<PathBuf> {
    let name = path_file_name(path)?;
    if name == "HEAD" && path.parent().and_then(path_file_name)? == ".git" {
        return path.parent()?.parent().map(Path::to_path_buf);
    }
    if is_lockfile_name(name) {
        return path.parent().map(Path::to_path_buf);
    }
    None
}

fn is_lockfile_name(name: &str) -> bool {
    matches!(
        name,
        "package-lock.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "Cargo.lock"
            | "Pipfile.lock"
            | "poetry.lock"
            | "uv.lock"
            | "go.sum"
    )
}

fn canonical_key(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

fn scan_error(message: String) -> RcleanError {
    RcleanError::Scan(ScanError::Generic(message))
}

#[cfg(test)]
mod tests;
