use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use chrono::Utc;
use notify::{Event, RecursiveMode, Watcher};

use crate::cli::WatchArgs;
use crate::error::{RcleanError, ScanError};
use crate::model::{Safety, ScanReport, format_bytes};
use crate::path_util::path_file_name;
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

    fn update_project(&mut self, root: &Path, report: &ScanReport) {
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
                print_diff(&project, &old, &CandidateMap::new());
            }
        }

        for project in &report.projects {
            let new = candidates_for(report, &project.path);
            let old = self.by_project.insert(project.path.clone(), new.clone());
            print_diff(&project.path, &old.unwrap_or_default(), &new);
        }
    }
}

pub fn run(args: WatchArgs) -> Result<ExitCode, RcleanError> {
    let options = args.common.to_scan_options()?;
    let roots = args.common.paths_or_current_dir();
    let every = parse::parse_duration(&args.every)?;

    let report = scan::scan(&roots, &options)?;
    output::print_table(&report);
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
    state.update_project(project_root, &report);
    write_timestamped_plan(args, &report)?;
    Ok(())
}

fn write_timestamped_plan(args: &WatchArgs, report: &ScanReport) -> Result<(), RcleanError> {
    let Some(base_path) = &args.common.write_plan else {
        return Ok(());
    };
    let path = timestamped_path(base_path);
    plan::write_action_plan(report, &path, args.common.include_caution, false, "trash")?;
    eprintln!("wrote action plan: {}", path.display());
    Ok(())
}

fn timestamped_path(path: &Path) -> PathBuf {
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("rclean-watch");
    let name = match path.extension().and_then(|value| value.to_str()) {
        Some(ext) => format!("{stem}-{stamp}.{ext}"),
        None => format!("{stem}-{stamp}"),
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

fn print_diff(project: &str, old: &CandidateMap, new: &CandidateMap) {
    for (path, candidate) in new {
        match old.get(path) {
            None => println!(
                "added: {path} ({}, {})",
                format_bytes(candidate.bytes),
                candidate.safety
            ),
            Some(previous) if previous.bytes != candidate.bytes => println!(
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
            println!("removed: {path}");
        }
    }
    if old == new {
        println!("refreshed: {project} (no candidate changes)");
    }
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
mod tests {
    use super::*;
    use crate::model::{ActivityInfo, Candidate, Category, ProjectReport, Summary};

    fn candidate_map(project: &str, bytes: u64) -> CandidateMap {
        BTreeMap::from([(
            format!("{project}/target"),
            CandidateSnapshot {
                bytes,
                safety: Safety::Safe,
            },
        )])
    }

    fn report_with_projects(projects: &[(&str, u64)]) -> ScanReport {
        ScanReport {
            schema_version: 1,
            tool_version: "test".to_string(),
            scanned_at: "2026-07-16T00:00:00Z".to_string(),
            roots: vec!["/workspace".to_string()],
            disk_attribution: None,
            warnings: Vec::new(),
            stale_after_days: 30,
            summary: Summary::default(),
            projects: projects
                .iter()
                .map(|(path, bytes)| ProjectReport {
                    path: (*path).to_string(),
                    kind: "Rust".to_string(),
                    markers: vec!["Cargo.toml".to_string()],
                    git: None,
                    activity: ActivityInfo {
                        last_modified: "2026-07-16T00:00:00Z".to_string(),
                        source: "test".to_string(),
                    },
                    candidates: vec![Candidate {
                        path: format!("{path}/target"),
                        name: "target".to_string(),
                        rule_id: "rust.target".to_string(),
                        category: Category::Build,
                        bytes: *bytes,
                        safety: Safety::Safe,
                        requires_sudo: false,
                        reasons: vec!["test fixture".to_string()],
                        warnings: Vec::new(),
                        restore_hint: "cargo build".to_string(),
                        risk_score: 0.0,
                        staleness_days: Some(1),
                    }],
                    total_bytes: *bytes,
                    project_bytes: *bytes,
                    artifact_percent: 100.0,
                })
                .collect(),
        }
    }

    #[test]
    fn maps_lockfile_to_project_root() {
        assert_eq!(
            project_root_for_lockfile(Path::new("/repo/app/package-lock.json")).unwrap(),
            PathBuf::from("/repo/app")
        );
        assert_eq!(
            project_root_for_lockfile(Path::new("/repo/app/.git/HEAD")).unwrap(),
            PathBuf::from("/repo/app")
        );
        assert!(project_root_for_lockfile(Path::new("/repo/app/package.json")).is_none());
    }

    #[test]
    fn reconciles_missing_projects_in_non_empty_polling_scope() {
        let mut state = WatchState {
            by_project: BTreeMap::from([
                ("/workspace/a".to_string(), candidate_map("/workspace/a", 1)),
                ("/workspace/b".to_string(), candidate_map("/workspace/b", 2)),
                ("/other/c".to_string(), candidate_map("/other/c", 3)),
            ]),
        };
        let outside_before = state.by_project["/other/c"].clone();

        state.update_project(
            Path::new("/workspace"),
            &report_with_projects(&[("/workspace/a", 4)]),
        );

        assert_eq!(
            state.by_project["/workspace/a"]["/workspace/a/target"].bytes,
            4
        );
        assert!(!state.by_project.contains_key("/workspace/b"));
        assert!(state.by_project["/other/c"] == outside_before);
    }

    #[test]
    fn empty_refresh_removes_descendants_without_string_prefix_collisions() {
        let mut state = WatchState {
            by_project: BTreeMap::from([
                ("/workspace".to_string(), candidate_map("/workspace", 1)),
                ("/workspace/a".to_string(), candidate_map("/workspace/a", 2)),
                ("/workspace/b".to_string(), candidate_map("/workspace/b", 3)),
                (
                    "/workspace-ab".to_string(),
                    candidate_map("/workspace-ab", 4),
                ),
            ]),
        };

        state.update_project(Path::new("/workspace"), &report_with_projects(&[]));

        assert!(!state.by_project.contains_key("/workspace"));
        assert!(!state.by_project.contains_key("/workspace/a"));
        assert!(!state.by_project.contains_key("/workspace/b"));
        assert!(state.by_project.contains_key("/workspace-ab"));
    }

    #[test]
    fn single_project_refresh_preserves_sibling_state() {
        let mut state = WatchState {
            by_project: BTreeMap::from([
                ("/workspace/a".to_string(), candidate_map("/workspace/a", 1)),
                ("/workspace/b".to_string(), candidate_map("/workspace/b", 2)),
                (
                    "/workspace/ab".to_string(),
                    candidate_map("/workspace/ab", 4),
                ),
            ]),
        };
        let sibling_before = state.by_project["/workspace/b"].clone();
        let prefix_sibling_before = state.by_project["/workspace/ab"].clone();

        state.update_project(
            Path::new("/workspace/a"),
            &report_with_projects(&[("/workspace/a", 3)]),
        );

        assert!(state.by_project["/workspace/b"] == sibling_before);
        assert!(state.by_project["/workspace/ab"] == prefix_sibling_before);
    }
}
