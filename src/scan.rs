use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use tracing::{debug, warn};

use crate::error::ScanError;
use crate::model::{
    ActivityInfo, Candidate, CandidateDraft, Category, Explanation, GitInfo, ProjectReport, Safety,
    ScanReport, Summary,
};
use crate::rules;
use crate::user_rules::UserRuleSet;

/// Per-directory immediate file-byte tally collected during `scan_dir`.
/// A project's source size is the fold of every entry under its `project_dir`,
/// which lets us drop the dedicated `project_source_size` walkdir pass.
type DirSizes = HashMap<PathBuf, u64>;

#[derive(Default)]
pub(crate) struct GitCache {
    by_dir: RefCell<HashMap<PathBuf, Option<GitInfo>>>,
}

impl GitCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn info_for(&self, dir: &Path) -> Option<GitInfo> {
        let cached_for_dir = self.by_dir.borrow().get(dir).cloned();
        if let Some(cached) = cached_for_dir {
            return cached;
        }

        let repo_root = match run_git_rev_parse(dir) {
            Some(root) => root,
            None => {
                self.by_dir.borrow_mut().insert(dir.to_path_buf(), None);
                return None;
            }
        };

        let root_path = PathBuf::from(&repo_root);
        let cached_for_root = self.by_dir.borrow().get(&root_path).cloned();
        if let Some(Some(info)) = cached_for_root {
            self.by_dir
                .borrow_mut()
                .insert(dir.to_path_buf(), Some(info.clone()));
            return Some(info);
        }

        let dirty = run_git_dirty(&root_path);
        let info = GitInfo { repo_root, dirty };
        let mut map = self.by_dir.borrow_mut();
        map.insert(root_path, Some(info.clone()));
        map.insert(dir.to_path_buf(), Some(info.clone()));
        Some(info)
    }
}

fn run_git_rev_parse(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let repo_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if repo_root.is_empty() {
        None
    } else {
        Some(repo_root)
    }
}

fn run_git_dirty(repo_root: &Path) -> bool {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["status", "--porcelain"])
        .output();
    matches!(output, Ok(o) if o.status.success() && !o.stdout.is_empty())
}

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

struct ScanContext<'a> {
    root: &'a Path,
    options: &'a ScanOptions,
    matcher: &'a IgnoreMatcher,
    user_rules: &'a UserRuleSet,
    git_cache: &'a GitCache,
    sizes: &'a mut DirSizes,
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
        let mut context = ScanContext {
            root: &root,
            options,
            matcher: &matcher,
            user_rules: &user_rules,
            git_cache: &git_cache,
            sizes: &mut sizes,
        };
        scan_dir(&root, 0, &mut context, &mut projects)?;
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

fn scan_dir(
    dir: &Path,
    depth: usize,
    context: &mut ScanContext<'_>,
    projects: &mut Vec<ProjectReport>,
) -> Result<(), ScanError> {
    if depth > context.options.max_depth || is_skip_dir(dir) {
        return Ok(());
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries.flatten().collect::<Vec<_>>(),
        Err(err) => {
            // v0.1.0 only emitted this with --verbose. Keep it at debug to
            // match the existing "noisy IO" level used by dir_size and
            // project_source_size, so non-verbose runs stay quiet.
            debug!(path = %dir.display(), error = %err, "skip directory");
            return Ok(());
        }
    };

    let mut drafts = Vec::new();
    let mut child_dirs = Vec::new();
    let mut file_bytes: u64 = 0;

    for entry in entries {
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        let is_symlink = metadata.file_type().is_symlink();

        if metadata.is_file() && !is_symlink {
            file_bytes = file_bytes.saturating_add(metadata.len());
            continue;
        }

        if !metadata.is_dir() && !is_symlink {
            continue;
        }

        let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };

        if rules::is_candidate_name(&name)
            && let Some(mut draft) = rules::classify_candidate(dir, &name, path.clone())
        {
            if context.matcher.is_ignored(&path, true) {
                continue;
            }
            apply_path_safety(context.root, &mut draft);
            if should_include(&draft, context.options) {
                drafts.push(draft);
            }
            continue;
        }

        // Builtin classifier missed. Give user rules from `.rclean.toml`
        // a chance — they can match arbitrary directory names like
        // `my_build_*` under user-declared `parent_markers`.
        if !context.user_rules.is_empty()
            && let Some(mut draft) = context.user_rules.classify(&name, dir)
        {
            if context.matcher.is_ignored(&path, true) {
                continue;
            }
            apply_path_safety(context.root, &mut draft);
            if should_include(&draft, context.options) {
                drafts.push(draft);
            }
            continue;
        }

        if metadata.is_dir() && !is_symlink && !is_skip_name(&name) {
            if context.matcher.is_ignored(&path, true) {
                continue;
            }
            child_dirs.push(path);
        }
    }

    context.sizes.insert(dir.to_path_buf(), file_bytes);

    for child in &child_dirs {
        scan_dir(child, depth + 1, context, projects)?;
    }

    if !drafts.is_empty() {
        let project = build_project_report(
            dir,
            context.root,
            drafts,
            context.options,
            context.git_cache,
            context.sizes,
        )?;
        if !project.candidates.is_empty() {
            projects.push(project);
        }
    }

    Ok(())
}

fn build_project_report(
    dir: &Path,
    _root: &Path,
    drafts: Vec<CandidateDraft>,
    options: &ScanOptions,
    git_cache: &GitCache,
    sizes: &DirSizes,
) -> Result<ProjectReport, ScanError> {
    let (kind, markers) = rules::detect_project_kind(dir);
    let git = git_cache.info_for(dir);
    let activity_time = project_activity(dir, options.max_depth).unwrap_or_else(SystemTime::now);

    if let Some(age) = options.older_than
        && SystemTime::now()
            .duration_since(activity_time)
            .unwrap_or_default()
            < age
    {
        return Ok(ProjectReport {
            path: dir.display().to_string(),
            kind,
            markers,
            git,
            activity: activity_info(activity_time, "computed"),
            candidates: Vec::new(),
            total_bytes: 0,
            project_bytes: 0,
            artifact_percent: 0.0,
        });
    }

    // Compute every draft's directory size in parallel. Each
    // `dir_size` walks an independent candidate subtree (e.g.
    // `node_modules`, `.next`, `.turbo`), so a project with N
    // candidates lets rayon split N walkdir traversals across
    // worker threads instead of running them sequentially.
    //
    // Blocked candidates short-circuit to 0 without walking. The
    // closure captures nothing mutable — `options.verbose` is
    // Copy and the path comes from each draft. The output
    // preserves input order so the subsequent zip is correct.
    use rayon::prelude::*;
    let draft_sizes: Vec<u64> = drafts
        .par_iter()
        .map(|draft| {
            if draft.safety == Safety::Blocked {
                0
            } else {
                dir_size(&draft.path, options.verbose)
            }
        })
        .collect();

    let mut candidates = Vec::new();
    for (mut draft, bytes) in drafts.into_iter().zip(draft_sizes) {
        if let Some(git) = &git
            && git.dirty
            && draft.safety == Safety::Safe
        {
            draft.safety = Safety::Caution;
            draft
                .warnings
                .push("project has uncommitted git changes".to_string());
        }

        if bytes < options.min_size && draft.safety != Safety::Blocked {
            continue;
        }

        let risk_score = compute_risk_score(git.as_ref(), activity_time, dir);

        candidates.push(Candidate {
            path: draft.path.display().to_string(),
            name: draft.name,
            rule_id: draft.rule_id,
            category: draft.category,
            bytes,
            safety: draft.safety,
            reasons: draft.reasons,
            warnings: draft.warnings,
            restore_hint: draft.restore_hint,
            risk_score,
        });
    }

    let total_bytes = candidates
        .iter()
        .filter(|candidate| candidate.safety != Safety::Blocked)
        .map(|candidate| candidate.bytes)
        .sum();
    let source_bytes = sum_subtree_bytes(dir, sizes);
    let project_bytes = source_bytes + total_bytes;
    let artifact_percent = if project_bytes == 0 {
        0.0
    } else {
        (total_bytes as f64 / project_bytes as f64) * 100.0
    };

    Ok(ProjectReport {
        path: dir.display().to_string(),
        kind,
        markers,
        git,
        activity: activity_info(activity_time, "computed"),
        candidates,
        total_bytes,
        project_bytes,
        artifact_percent,
    })
}

fn should_include(draft: &CandidateDraft, options: &ScanOptions) -> bool {
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

fn apply_path_safety(root: &Path, draft: &mut CandidateDraft) {
    let metadata = fs::symlink_metadata(&draft.path);
    if metadata
        .as_ref()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        draft.safety = Safety::Blocked;
        draft.warnings.push("candidate is a symlink".to_string());
        return;
    }

    // Global rules (e.g. `xcode.derived_data`) target paths that
    // live *inside* the user's Library / runtime tree by design.
    // Their classifier already establishes that the path is a
    // rebuildable cache, so the generic runtime/system-path block
    // would otherwise hide them. is_global_rule() is the
    // explicit opt-out list.
    if !rules::is_global_rule(&draft.rule_id) && is_runtime_or_system_path(&draft.path) {
        draft.safety = Safety::Blocked;
        draft
            .warnings
            .push("candidate is inside a protected runtime or system path".to_string());
        return;
    }

    if root != Path::new(".") {
        let root = root.canonicalize().ok();
        let candidate = draft.path.canonicalize().ok();
        if let (Some(root), Some(candidate)) = (root, candidate)
            && !candidate.starts_with(root)
        {
            draft.safety = Safety::Blocked;
            draft
                .warnings
                .push("candidate resolves outside the scan root".to_string());
        }
    }
}

fn build_summary(projects: &[ProjectReport]) -> Summary {
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
                Safety::Unknown => {}
            }
        }
    }

    summary
}

fn activity_info(time: SystemTime, source: &str) -> ActivityInfo {
    let datetime: DateTime<Utc> = time.into();
    ActivityInfo {
        last_modified: datetime.to_rfc3339(),
        source: source.to_string(),
    }
}

fn project_activity(project_dir: &Path, max_depth: usize) -> Option<SystemTime> {
    let mut newest = fs::metadata(project_dir)
        .and_then(|metadata| metadata.modified())
        .ok();

    for entry in walkdir::WalkDir::new(project_dir)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry
                .file_name()
                .to_str()
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

fn dir_size(path: &Path, _verbose: bool) -> u64 {
    let mut total: u64 = 0;
    for result in walkdir::WalkDir::new(path).follow_links(false) {
        let entry = match result {
            Ok(entry) => entry,
            Err(err) => {
                debug!(path = %path.display(), error = %err, "dir_size walk error");
                continue;
            }
        };
        match entry.metadata() {
            Ok(metadata) if metadata.is_file() => {
                total = total.saturating_add(metadata.len());
            }
            Ok(_) => {}
            Err(err) => {
                debug!(path = %entry.path().display(), error = %err, "dir_size metadata error");
            }
        }
    }
    total
}

/// Folds every per-directory `file_bytes` tally that `scan_dir` already
/// collected for paths under `project_dir`. Candidate subtrees are absent
/// from the map (scan_dir doesn't recurse into them — `dir_size` handles
/// those separately, unbounded), and skipped/excluded names never make it
/// into the map either.
fn sum_subtree_bytes(project_dir: &Path, sizes: &DirSizes) -> u64 {
    let mut total: u64 = 0;
    for (path, bytes) in sizes {
        if path == project_dir || path.starts_with(project_dir) {
            total = total.saturating_add(*bytes);
        }
    }
    total
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
fn compute_risk_score(git: Option<&GitInfo>, activity_time: SystemTime, project_dir: &Path) -> f32 {
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

fn is_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_skip_name)
}

fn is_skip_name(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".hg"
            | ".svn"
            | ".Trash"
            | "Library"
            | "Applications"
            | ".cargo"
            | ".rustup"
            | ".nvm"
            | ".fnm"
            | ".pyenv"
            | ".sdkman"
            | ".rbenv"
            | ".conda"
            | ".terraform"
    )
}

pub(crate) fn is_runtime_or_system_path(path: &Path) -> bool {
    let protected: HashSet<&str> = [
        ".cargo",
        ".rustup",
        ".nvm",
        ".fnm",
        ".pyenv",
        ".sdkman",
        ".rbenv",
        ".conda",
        "Library",
        "Applications",
        ".Trash",
    ]
    .into_iter()
    .collect();

    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|name| protected.contains(name))
    })
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
