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

mod git_cache;
mod project;
mod safety;
mod sizer;
mod walker;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use chrono::Utc;
use tracing::warn;

use crate::error::ScanError;
use crate::model::{CandidateDraft, Category, Explanation, Safety, ScanReport};
use crate::rules;
use crate::user_rules::UserRuleSet;

pub(crate) use git_cache::GitCache;
pub(crate) use project::{build_project_report, build_summary, compute_risk_score, project_activity};
pub(crate) use safety::{apply_path_safety, is_runtime_or_system_path};
pub(crate) use sizer::DirSizes;
pub(crate) use walker::{WalkScratch, walk_parallel};

#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    pub max_depth: usize,
    pub min_size: u64,
    pub older_than: Option<Duration>,
    pub categories: Option<Vec<Category>>,
    pub rule_ids: Option<Vec<String>>,
    pub include_blocked: bool,
    pub verbose: bool,
    /// Extra gitignore-style globs from `--ignore` CLI flags, layered on
    /// top of any `.rcleanignore` file at the scan root.
    pub ignore_globs: Vec<String>,
}

/// Compiled .gitignore-style matcher built from each scan root's
/// `.rcleanignore` file plus any `--ignore <glob>` flags. Candidates whose
/// path matches an ignore pattern are dropped before classification —
/// they never appear in the report, plan, or table.
pub(crate) struct IgnoreMatcher {
    matcher: ignore::gitignore::Gitignore,
}

impl IgnoreMatcher {
    pub(crate) fn build(root: &Path, extra_globs: &[String]) -> Self {
        let mut builder = ignore::gitignore::GitignoreBuilder::new(root);
        let rcleanignore = root.join(".rcleanignore");
        if rcleanignore.is_file()
            && let Some(err) = builder.add(&rcleanignore)
        {
            warn!(path = %rcleanignore.display(), error = %err, "failed to load .rcleanignore");
        }
        for glob in extra_globs {
            if let Err(err) = builder.add_line(None, glob) {
                warn!(glob = %glob, error = %err, "invalid --ignore glob");
            }
        }
        let matcher = match builder.build() {
            Ok(m) => m,
            Err(err) => {
                warn!(root = %root.display(), error = %err, "failed to build .rcleanignore matcher");
                ignore::gitignore::Gitignore::empty()
            }
        };
        Self { matcher }
    }

    pub(crate) fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        self.matcher.matched(path, is_dir).is_ignore()
    }
}

pub fn scan(paths: &[PathBuf], options: &ScanOptions) -> Result<ScanReport, ScanError> {
    let mut roots = Vec::new();
    let mut projects = Vec::new();
    let git_cache = GitCache::new();
    let mut sizes: DirSizes = HashMap::new();

    for path in paths {
        let root = path
            .canonicalize()
            .map_err(|source| ScanError::CanonicalizeRoot {
                path: path.clone(),
                source,
            })?;
        roots.push(root.display().to_string());
        let user_rules = UserRuleSet::load_from_root(&root);
        let matcher = IgnoreMatcher::build(&root, &options.ignore_globs);

        // Phase 1: parallel walk collects (project_dir, drafts) and
        // per-dir file_bytes into thread-safe accumulators.
        let walk = WalkScratch::new();
        walk_parallel(&root, options, &matcher, &user_rules, &walk);

        let (drafts_by_project, walk_sizes) = walk.into_inner();
        for (dir, bytes) in walk_sizes {
            *sizes.entry(dir).or_insert(0) += bytes;
        }

        // Phase 2: serial post-processing per project so dir_size,
        // git_info, and risk_score all see consistent state. Sort
        // project_dirs deterministically so the output order is
        // stable across runs.
        let mut project_dirs: Vec<PathBuf> = drafts_by_project.keys().cloned().collect();
        project_dirs.sort();
        for project_dir in project_dirs {
            let drafts = drafts_by_project
                .get(&project_dir)
                .cloned()
                .unwrap_or_default();
            if drafts.is_empty() {
                continue;
            }
            let project = build_project_report(
                &project_dir,
                &root,
                drafts,
                options,
                &git_cache,
                &sizes,
            )?;
            if !project.candidates.is_empty() {
                projects.push(project);
            }
        }
    }

    projects.sort_by_key(|p| std::cmp::Reverse(p.total_bytes));
    let summary = build_summary(&projects);

    Ok(ScanReport {
        schema_version: 1,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        scanned_at: Utc::now().to_rfc3339(),
        roots,
        summary,
        projects,
    })
}

pub fn explain_path(path: &Path) -> Result<Explanation, ScanError> {
    let parent = path
        .parent()
        .ok_or_else(|| ScanError::Generic(format!("{} has no parent directory", path.display())))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
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

    // Same risk signal the scan path emits. `parent` is the project
    // dir; max_depth 6 matches v0.1.0's default `--depth`. The GitCache
    // lookup shells out once here — explain is single-shot, not a scan
    // loop. `project_activity` fallback to `now()` conservatively trips
    // the recent-mtime axis (+0.25); scan path uses the same fallback.
    let git = GitCache::new().info_for(parent);
    let activity_time = project_activity(parent, 6).unwrap_or_else(SystemTime::now);
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
        Safety::Unknown => false,
    }
}

#[cfg(test)]
mod tests;
