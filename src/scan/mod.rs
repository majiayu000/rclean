//! Scan-phase entry points and shared types.
//!
//! The scan pipeline is split across this directory module:
//!
//! - [`walker`] — phase 1: parallel `ignore::WalkParallel` traversal
//!   that classifies candidate directories and tallies per-dir
//!   file bytes.
//! - [`sizer`] — `DirSizes` type, the per-candidate `dir_size`
//!   unbounded walk, and the `sum_subtree_bytes` fold.
//! - [`safety`] — symlink / system-path / scan-root checks plus
//!   the `is_skip_dir` / `is_skip_name` pruning predicates.
//! - [`git_cache`] — per-repo `git status` cache shared across
//!   worker threads.
//! - [`project`] — phase 2: turn drafts into `ProjectReport`,
//!   compute summary, activity, and risk score.
//!
//! `IgnoreMatcher` and the `--ignore` glob / `.rcleanignore`
//! gitignore-style filter live here next to `ScanOptions` because
//! they are the only types every submodule needs to import.

mod disk;
mod git_cache;
mod progress;
mod project;
mod safety;
mod sizer;
mod walker;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};

use chrono::Utc;
use tracing::warn;

use crate::error::ScanError;
use crate::model::{CandidateDraft, Category, Explanation, Safety, ScanReport, ScanWarning};
use crate::path_util::{path_file_name, path_file_name_string};
use crate::rules;
use crate::user_rules::UserRuleSet;

pub const DEFAULT_ACTIVITY_DEPTH: usize = 6;
pub const DEFAULT_GIT_TIMEOUT_SECS: u64 = 5;
pub const DEFAULT_GIT_TIMEOUT: Duration = Duration::from_secs(DEFAULT_GIT_TIMEOUT_SECS);

pub(crate) use git_cache::GitCache;
pub(crate) use progress::progress_enabled;
pub(crate) use project::{
    build_project_report, build_summary, compute_risk_score, project_activities, project_activity,
};
pub(crate) use safety::{
    apply_path_safety, dangerous_link_kind, is_docker_storage_path, is_protected_user_data_path,
    is_runtime_or_system_path,
};
pub(crate) use sizer::SourceSizeIndex;
pub(crate) use walker::{WalkScratch, walk_parallel};

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub max_depth: usize,
    pub min_size: u64,
    pub older_than: Option<Duration>,
    pub categories: Option<Vec<Category>>,
    pub rule_ids: Option<Vec<String>>,
    pub include_blocked: bool,
    pub verbose: bool,
    pub disk_attribution: bool,
    /// True when roots came from explicit `--tmp` expansion. Whole
    /// temporary worktree fallbacks are only allowed in this mode.
    pub tmp_roots: bool,
    /// True when roots came from explicit `--system` expansion. System
    /// roots are exact report-only anchors, not traversal roots.
    pub system_roots: bool,
    /// Extra gitignore-style globs from `--ignore` CLI flags, layered on
    /// top of any `.rcleanignore` file at the scan root.
    pub ignore_globs: Vec<String>,
    /// Timeout for each git metadata subprocess. `None` disables git checks.
    pub git_timeout: Option<Duration>,
    /// Stream a single-line progress indicator to stderr for scans
    /// longer than ~1s. Decided by the CLI layer; never touches stdout.
    pub progress: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_depth: 0,
            min_size: 0,
            older_than: None,
            categories: None,
            rule_ids: None,
            include_blocked: false,
            verbose: false,
            disk_attribution: false,
            tmp_roots: false,
            system_roots: false,
            ignore_globs: Vec::new(),
            git_timeout: Some(DEFAULT_GIT_TIMEOUT),
            progress: false,
        }
    }
}

/// Compiled .gitignore-style matcher built from each scan root's
/// `.rcleanignore` file plus any `--ignore <glob>` flags. Candidates whose
/// path matches an ignore pattern are dropped before classification —
/// they never appear in the report, plan, or table.
pub(crate) struct IgnoreMatcher {
    matcher: ignore::gitignore::Gitignore,
}

impl IgnoreMatcher {
    pub(crate) fn build(
        root: &Path,
        extra_globs: &[String],
    ) -> Result<(Self, Vec<ScanWarning>), ScanError> {
        let mut warnings = Vec::new();
        let mut builder = ignore::gitignore::GitignoreBuilder::new(root);
        let rcleanignore = root.join(".rcleanignore");
        if rcleanignore.is_file()
            && let Some(err) = builder.add(&rcleanignore)
        {
            warn!(path = %rcleanignore.display(), error = %err, "failed to load .rcleanignore");
            warnings.push(ScanWarning::IgnoreFileLoad {
                path: rcleanignore,
                error: err.to_string(),
            });
        }
        for glob in extra_globs {
            if let Err(err) = builder.add_line(None, glob) {
                return Err(ScanError::Generic(format!(
                    "invalid --ignore glob '{glob}': {err}"
                )));
            }
        }
        let matcher = builder.build().map_err(|err| {
            ScanError::Generic(format!(
                "failed to build ignore matcher for {}: {err}",
                root.display()
            ))
        })?;
        Ok((Self { matcher }, warnings))
    }

    pub(crate) fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        self.matcher.matched(path, is_dir).is_ignore()
    }
}

fn canonical_scan_roots(paths: &[PathBuf]) -> Result<Vec<PathBuf>, ScanError> {
    let mut seen = HashSet::new();
    let mut roots = Vec::new();
    for path in paths {
        let root = path
            .canonicalize()
            .map_err(|source| ScanError::CanonicalizeRoot {
                path: path.clone(),
                source,
            })?;
        if seen.insert(root.clone()) {
            roots.push(root);
        }
    }
    Ok(roots)
}

pub fn scan(paths: &[PathBuf], options: &ScanOptions) -> Result<ScanReport, ScanError> {
    let canonical_roots = canonical_scan_roots(paths)?;
    let mut roots = Vec::with_capacity(canonical_roots.len());
    let mut stale_after_days: Option<u64> = None;
    let reporter = options.progress.then(progress::ProgressReporter::start);
    let progress_counters = reporter.as_ref().map(progress::ProgressReporter::counters);
    let mut projects = Vec::new();
    let mut warnings = Vec::new();
    let git_cache = GitCache::with_timeout(options.git_timeout);

    for root in canonical_roots {
        roots.push(root.display().to_string());
        let user_rules = UserRuleSet::load_from_root(&root);
        if stale_after_days.is_none() {
            stale_after_days = user_rules.stale_after_days();
        }
        let (matcher, matcher_warnings) = IgnoreMatcher::build(&root, &options.ignore_globs)?;
        warnings.extend(matcher_warnings);

        let (drafts_by_project, walk_sizes) = if options.system_roots {
            let mut drafts_by_project = HashMap::new();
            add_system_root_candidate(&root, options, &mut drafts_by_project);
            (drafts_by_project, HashMap::new())
        } else {
            // Phase 1: parallel walk collects (project_dir, drafts) and
            // per-dir file_bytes into thread-safe accumulators.
            let walk = WalkScratch::new_with_progress(progress_counters.clone());
            walk_parallel(&root, options, &matcher, &user_rules, &walk);

            let (mut drafts_by_project, walk_sizes, walk_warnings) = walk.into_inner()?;
            warnings.extend(walk_warnings);
            add_tmp_worktree_fallbacks(
                &root,
                options,
                &matcher,
                &mut drafts_by_project,
                &mut warnings,
            )?;
            (drafts_by_project, walk_sizes)
        };
        let source_sizes = SourceSizeIndex::from_dir_sizes(&walk_sizes);

        // Phase 2: serial post-processing per project so dir_size,
        // git_info, and risk_score all see consistent state. Sort
        // project_dirs deterministically so the output order is
        // stable across runs.
        let mut project_dirs: Vec<PathBuf> = drafts_by_project.keys().cloned().collect();
        project_dirs.sort();
        let activity_times = project_activities(&project_dirs, options.max_depth);
        for (project_dir, activity_time) in project_dirs.into_iter().zip(activity_times) {
            let drafts = drafts_by_project
                .get(&project_dir)
                .cloned()
                .unwrap_or_default();
            if drafts.is_empty() {
                continue;
            }
            let (project, sizing_warnings) = build_project_report(
                &project_dir,
                &root,
                drafts,
                options,
                &git_cache,
                &source_sizes,
                activity_time,
            )?;
            warnings.extend(sizing_warnings);
            if let Some(counters) = &progress_counters {
                counters.add_project();
            }
            if !project.candidates.is_empty() {
                projects.push(project);
            }
        }
    }

    projects.sort_by_key(|p| std::cmp::Reverse(p.total_bytes));
    let summary = build_summary(&projects);

    if let Some(reporter) = reporter {
        reporter.finish();
    }

    Ok(ScanReport {
        schema_version: 1,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        scanned_at: Utc::now().to_rfc3339(),
        roots,
        disk_attribution: options
            .disk_attribution
            .then(disk::collect_disk_attribution)
            .flatten(),
        warnings,
        stale_after_days: stale_after_days.unwrap_or_else(crate::model::default_stale_after_days),
        summary,
        projects,
    })
}

fn add_system_root_candidate(
    root: &Path,
    options: &ScanOptions,
    drafts_by_project: &mut HashMap<PathBuf, Vec<CandidateDraft>>,
) {
    if !options.system_roots {
        return;
    }
    let Some(name) = path_file_name_string(root) else {
        return;
    };
    let Some(parent) = root.parent() else {
        return;
    };
    let Some(mut draft) = rules::classify_candidate(parent, &name, root.to_path_buf()) else {
        return;
    };
    apply_path_safety(root, &mut draft);
    if should_include(&draft, options) {
        drafts_by_project
            .entry(root.to_path_buf())
            .or_default()
            .push(draft);
    }
}

fn add_tmp_worktree_fallbacks(
    root: &Path,
    options: &ScanOptions,
    matcher: &IgnoreMatcher,
    drafts_by_project: &mut HashMap<PathBuf, Vec<CandidateDraft>>,
    warnings: &mut Vec<ScanWarning>,
) -> Result<(), ScanError> {
    if !options.tmp_roots || options.max_depth == 0 {
        return Ok(());
    }

    let top_levels_with_nested_cleanable =
        top_level_children_with_cleanable_candidates(root, drafts_by_project);
    let entries = fs::read_dir(root).map_err(|source| {
        ScanError::Generic(format!(
            "failed to read tmp root {} for worktree fallback scan: {source}",
            root.display()
        ))
    })?;
    let mut children = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => children.push(entry.path()),
            Err(err) => warnings.push(ScanWarning::WalkError {
                path: Some(root.to_path_buf()),
                error: err.to_string(),
            }),
        }
    }
    children.sort();

    for child in children {
        if top_levels_with_nested_cleanable.contains(&child) || matcher.is_ignored(&child, true) {
            continue;
        }
        let Some(name) = path_file_name_string(&child) else {
            continue;
        };
        let metadata = match fs::symlink_metadata(&child) {
            Ok(metadata) => metadata,
            Err(err) => {
                warnings.push(ScanWarning::MetadataError {
                    path: child,
                    error: err.to_string(),
                });
                continue;
            }
        };
        if !metadata.is_dir() && !metadata.file_type().is_symlink() {
            continue;
        }

        let Some(mut draft) = rules::classify_agent_tmp_worktree(root, &name, &child) else {
            continue;
        };
        apply_path_safety(root, &mut draft);
        if should_include(&draft, options) {
            drafts_by_project.entry(child).or_default().push(draft);
        }
    }

    Ok(())
}

fn top_level_children_with_cleanable_candidates(
    root: &Path,
    drafts_by_project: &HashMap<PathBuf, Vec<CandidateDraft>>,
) -> HashSet<PathBuf> {
    drafts_by_project
        .values()
        .flat_map(|drafts| drafts.iter())
        .filter(|draft| matches!(draft.safety, Safety::Safe | Safety::Caution))
        .filter_map(|draft| top_level_child(root, &draft.path))
        .collect()
}

fn top_level_child(root: &Path, path: &Path) -> Option<PathBuf> {
    let mut components = path.strip_prefix(root).ok()?.components();
    let Component::Normal(name) = components.next()? else {
        return None;
    };
    Some(root.join(name))
}

pub fn explain_path_with_activity_depth(
    path: &Path,
    activity_depth: usize,
) -> Result<Explanation, ScanError> {
    let parent = path
        .parent()
        .ok_or_else(|| ScanError::Generic(format!("{} has no parent directory", path.display())))?;
    let name = path_file_name(path)
        .ok_or_else(|| ScanError::Generic(format!("{} has no valid file name", path.display())))?;

    let Some(mut draft) = rules::classify_candidate(parent, name, path.to_path_buf()) else {
        return Ok(Explanation {
            path: path.to_path_buf(),
            safety: Safety::Unknown,
            rule_id: None,
            category: None,
            reasons: vec!["no built-in rule matched this path".to_string()],
            warnings: Vec::new(),
            restore_hint: None,
            risk_score: None,
        });
    };

    apply_path_safety(Path::new("."), &mut draft);

    // Same risk signal the scan path emits. The GitCache lookup shells
    // out once here — explain is single-shot, not a scan loop.
    // `project_activity` fallback to `now()` conservatively trips the
    // recent-mtime axis (+0.25); scan path uses the same fallback.
    let git = GitCache::new().info_for(parent);
    let activity_time = project_activity(parent, activity_depth).unwrap_or_else(SystemTime::now);
    let risk_score = compute_risk_score(git.as_ref(), activity_time, parent);

    Ok(Explanation {
        path: path.to_path_buf(),
        safety: draft.safety,
        rule_id: Some(draft.rule_id),
        category: Some(draft.category),
        reasons: draft.reasons,
        warnings: draft.warnings,
        restore_hint: Some(draft.restore_hint),
        risk_score: Some(risk_score),
    })
}

pub(crate) fn should_include(draft: &CandidateDraft, options: &ScanOptions) -> bool {
    if let Some(categories) = &options.categories
        && !categories.contains(&draft.category)
    {
        return false;
    }
    if let Some(rule_ids) = &options.rule_ids
        && !rule_ids.contains(&draft.rule_id)
    {
        return false;
    }
    match draft.safety {
        Safety::Safe => true,
        Safety::Caution => true,
        Safety::Blocked => options.include_blocked,
        // ReportOnly is **always reported** so the user is aware of
        // the path's size, but selection paths (clean.rs, plan.rs)
        // refuse to act on it even with --include-blocked.
        Safety::ReportOnly => true,
        Safety::Unknown => false,
    }
}

#[cfg(test)]
mod tests;
