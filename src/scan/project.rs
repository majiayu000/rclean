//! Phase 2 of `scan()`: turn classified drafts into `ProjectReport`s.
//!
//! For each project directory the walker grouped candidates under,
//! [`build_project_report`] resolves git status (via [`GitCache`]),
//! computes project activity, demotes Safe → Caution when the repo
//! is dirty, expands candidate byte sizes (`dir_size`), and
//! optionally suppresses the project entirely if `--older-than`
//! filters it out.
//!
//! The risk-score logic lives here too because it consumes the same
//! `GitInfo` / activity inputs and is only emitted alongside the
//! project report.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use rayon::prelude::*;

use crate::error::ScanError;
use crate::model::{
    ActivityInfo, Candidate, CandidateDraft, GitInfo, ProjectReport, Safety, ScanWarning, Summary,
};
use crate::path_util::path_file_name;
use crate::rules;

use super::ScanOptions;
use super::git_cache::GitCache;
use super::safety::is_skip_name;
use super::sizer::{self, SourceSizeIndex};

pub(crate) fn build_project_report(
    dir: &Path,
    _root: &Path,
    drafts: Vec<CandidateDraft>,
    options: &ScanOptions,
    git_cache: &GitCache,
    source_sizes: &SourceSizeIndex,
    activity_time: SystemTime,
) -> Result<(ProjectReport, Vec<ScanWarning>), ScanError> {
    let (kind, markers) = rules::detect_project_kind(dir);
    let git = git_cache.info_for(dir);

    if let Some(age) = options.older_than
        && SystemTime::now()
            .duration_since(activity_time)
            .unwrap_or_default()
            < age
    {
        return Ok((
            ProjectReport {
                path: dir.display().to_string(),
                kind,
                markers,
                git,
                activity: activity_info(activity_time, "computed"),
                candidates: Vec::new(),
                total_bytes: 0,
                project_bytes: 0,
                artifact_percent: 0.0,
            },
            Vec::new(),
        ));
    }

    let size_summary = sizer::summarize(dir, &drafts, source_sizes, options.verbose);

    let mut candidates = Vec::new();
    for (mut draft, bytes) in drafts.into_iter().zip(size_summary.candidate_bytes) {
        if let Some(git) = &git
            && git.dirty
            && draft.safety == Safety::Safe
        {
            draft.safety = Safety::Caution;
            draft
                .warnings
                .push("project has uncommitted git changes".to_string());
        }

        if bytes < options.min_size
            && draft.safety != Safety::Blocked
            && draft.safety != Safety::ReportOnly
        {
            continue;
        }

        let risk_score = compute_risk_score(git.as_ref(), activity_time, dir);
        let requires_sudo = rules::requires_sudo(&draft.rule_id);
        let staleness_days = SystemTime::now()
            .duration_since(activity_time)
            .ok()
            .map(|age| age.as_secs() / 86_400);

        candidates.push(Candidate {
            path: draft.path.display().to_string(),
            name: draft.name,
            rule_id: draft.rule_id,
            category: draft.category,
            bytes,
            safety: draft.safety,
            requires_sudo,
            reasons: draft.reasons,
            warnings: draft.warnings,
            restore_hint: draft.restore_hint,
            risk_score,
            staleness_days,
        });
    }

    let total_bytes = candidates
        .iter()
        .filter(|candidate| {
            candidate.safety != Safety::Blocked && candidate.safety != Safety::ReportOnly
        })
        .map(|candidate| candidate.bytes)
        .sum();
    let source_bytes = size_summary.source_bytes;
    let project_bytes = source_bytes + total_bytes;
    let artifact_percent = if project_bytes == 0 {
        0.0
    } else {
        (total_bytes as f64 / project_bytes as f64) * 100.0
    };

    Ok((
        ProjectReport {
            path: dir.display().to_string(),
            kind,
            markers,
            git,
            activity: activity_info(activity_time, "computed"),
            candidates,
            total_bytes,
            project_bytes,
            artifact_percent,
        },
        size_summary.warnings,
    ))
}

pub(crate) fn build_summary(projects: &[ProjectReport]) -> Summary {
    let mut summary = Summary {
        projects_scanned: projects.len(),
        projects_with_candidates: projects
            .iter()
            .filter(|project| !project.candidates.is_empty())
            .count(),
        ..Summary::default()
    };

    for project in projects {
        for candidate in &project.candidates {
            summary.candidates += 1;
            match candidate.safety {
                Safety::Safe => {
                    summary.safe_candidates += 1;
                    summary.total_bytes += candidate.bytes;
                }
                Safety::Caution => {
                    summary.caution_candidates += 1;
                    summary.total_bytes += candidate.bytes;
                }
                Safety::Blocked => summary.blocked_candidates += 1,
                Safety::ReportOnly => summary.report_only_candidates += 1,
                Safety::Unknown => {}
            }
        }
    }

    summary
}

pub(crate) fn activity_info(time: SystemTime, source: &str) -> ActivityInfo {
    let datetime: DateTime<Utc> = time.into();
    ActivityInfo {
        last_modified: datetime.to_rfc3339(),
        source: source.to_string(),
    }
}

pub(crate) fn project_activity(project_dir: &Path, max_depth: usize) -> Option<SystemTime> {
    let mut newest = fs::metadata(project_dir)
        .and_then(|metadata| metadata.modified())
        .ok();

    for entry in walkdir::WalkDir::new(project_dir)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            path_file_name(entry.path())
                .is_none_or(|name| !is_skip_name(name) && !rules::is_candidate_name(name))
        })
        .flatten()
    {
        if entry.file_type().is_dir() {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if newest.is_none_or(|current| modified > current) {
            newest = Some(modified);
        }
    }

    newest
}

pub(crate) fn project_activities(project_dirs: &[PathBuf], max_depth: usize) -> Vec<SystemTime> {
    match project_dirs {
        [] => Vec::new(),
        [only] => vec![resolved_project_activity(only, max_depth)],
        many => many
            .par_iter()
            .map(|dir| resolved_project_activity(dir, max_depth))
            .collect(),
    }
}

fn resolved_project_activity(project_dir: &Path, max_depth: usize) -> SystemTime {
    project_activity(project_dir, max_depth).unwrap_or_else(SystemTime::now)
}

/// Composite risk-score signal for a candidate inside `project_dir`.
///
/// Range in the **final** formula is `[0.0, 1.0]`. The current
/// implementation reaches a maximum of **0.85** because the
/// `root_boundary` axis (weight 0.15) is deferred to a follow-up PR
/// that wires up cross-filesystem + cwd-ancestor detection. Until then,
/// consumers should treat 0.85 as "every implemented axis tripped" and
/// not assume 0.85 means "every conceivable risk axis tripped".
///
/// First-cut weights match `docs/specs/v0.1.x-roadmap.md` §4.6:
///   - dirty git worktree         -> 0.40
///   - project activity < 7 days  -> 0.25
///   - no lockfile present        -> 0.20
///   - root-boundary signal       -> 0.15  (deferred — always 0.0 here)
///
/// The weight slot stays in the formula so safe/caution thresholds in
/// downstream consumers (TUI coloring, agent plan scoring) don't have to
/// shift when the boundary axis lights up.
///
/// Note: this signal is independent of `safety` tier promotion. A dirty
/// git worktree both (a) demotes Safe → Caution in `build_project_report`
/// and (b) contributes 0.40 to `risk_score` here. The two are intentional
/// duplicates: safety is an operational gate (controls auto-selection),
/// risk_score is an advisory analytical signal (controls coloring /
/// scoring). Don't collapse them into one.
pub(crate) fn compute_risk_score(
    git: Option<&GitInfo>,
    activity_time: SystemTime,
    project_dir: &Path,
) -> f32 {
    let dirty_git: f32 = match git {
        Some(info) if info.dirty => 1.0,
        _ => 0.0,
    };
    let recent_mtime: f32 = match SystemTime::now().duration_since(activity_time) {
        Ok(age) if age < Duration::from_secs(7 * 24 * 60 * 60) => 1.0,
        _ => 0.0,
    };
    let no_lockfile: f32 = if has_lockfile(project_dir) { 0.0 } else { 1.0 };
    let root_boundary: f32 = 0.0;

    let score = dirty_git * 0.40 + recent_mtime * 0.25 + no_lockfile * 0.20 + root_boundary * 0.15;
    score.clamp(0.0, 1.0)
}

fn has_lockfile(project_dir: &Path) -> bool {
    const LOCKFILES: &[&str] = &[
        "Cargo.lock",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "bun.lockb",
        "Pipfile.lock",
        "poetry.lock",
        "uv.lock",
        "go.sum",
        "Gemfile.lock",
        "composer.lock",
        "pubspec.lock",
    ];
    LOCKFILES
        .iter()
        .any(|name| project_dir.join(name).is_file())
}

#[cfg(test)]
mod tests {
    use std::fs::{self, FileTimes, OpenOptions};

    use tempfile::TempDir;

    use super::*;

    fn write_with_modified(path: &Path, modified: SystemTime) {
        fs::write(path, b"source").unwrap();
        OpenOptions::new()
            .write(true)
            .open(path)
            .unwrap()
            .set_times(FileTimes::new().set_modified(modified))
            .unwrap();
    }

    #[test]
    fn project_activities_handles_empty_and_single_inputs() {
        assert!(project_activities(&[], 6).is_empty());

        let temp = TempDir::new().unwrap();
        let project = temp.path().join("single");
        fs::create_dir(&project).unwrap();
        let expected = SystemTime::UNIX_EPOCH + Duration::from_secs(4_000_000_000);
        write_with_modified(&project.join("source.rs"), expected);

        assert_eq!(project_activities(&[project], 6), vec![expected]);
    }

    #[test]
    fn project_activities_preserves_input_order() {
        let temp = TempDir::new().unwrap();
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();

        let first_time = SystemTime::UNIX_EPOCH + Duration::from_secs(4_000_000_000);
        let second_time = first_time + Duration::from_secs(60);
        write_with_modified(&first.join("source.rs"), first_time);
        write_with_modified(&second.join("source.rs"), second_time);

        assert_eq!(
            project_activities(&[second, first], 6),
            vec![second_time, first_time]
        );
    }

    #[test]
    fn project_activities_match_serial_traversal_boundaries() {
        let temp = TempDir::new().unwrap();
        let mut projects = Vec::new();

        for name in ["alpha", "beta"] {
            let project = temp.path().join(name);
            fs::create_dir_all(project.join("src").join("deep")).unwrap();
            fs::create_dir(project.join("node_modules")).unwrap();
            fs::create_dir(project.join(".git")).unwrap();
            fs::write(project.join("src").join("visible.rs"), b"visible").unwrap();
            fs::write(
                project.join("src").join("deep").join("too_deep.rs"),
                b"deep",
            )
            .unwrap();
            fs::write(project.join("node_modules").join("artifact"), b"artifact").unwrap();
            fs::write(project.join(".git").join("index"), b"git").unwrap();
            let external = temp.path().join(format!("{name}-external.rs"));
            fs::write(&external, b"external").unwrap();
            #[cfg(unix)]
            std::os::unix::fs::symlink(&external, project.join("linked-source.rs")).unwrap();
            #[cfg(windows)]
            std::os::windows::fs::symlink_file(&external, project.join("linked-source.rs"))
                .unwrap();
            projects.push(project);
        }

        let serial = projects
            .iter()
            .map(|project| project_activity(project, 2).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(project_activities(&projects, 2), serial);
    }

    #[test]
    fn project_activities_preserves_missing_project_slots() {
        let temp = TempDir::new().unwrap();
        let before = SystemTime::now();
        let missing = vec![
            temp.path().join("missing-project-a"),
            temp.path().join("missing-project-b"),
        ];

        let activities = project_activities(&missing, 6);
        let after = SystemTime::now();

        assert_eq!(activities.len(), missing.len());
        assert!(
            activities
                .iter()
                .all(|activity| *activity >= before && *activity <= after)
        );
    }
}
