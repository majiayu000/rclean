use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};

use crate::model::{
    ActivityInfo, Candidate, CandidateDraft, Category, Explanation, GitInfo, ProjectReport, Safety,
    ScanReport, Summary,
};
use crate::rules;

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub max_depth: usize,
    pub min_size: u64,
    pub older_than: Option<Duration>,
    pub categories: Option<Vec<Category>>,
    pub rule_ids: Option<Vec<String>>,
    pub include_blocked: bool,
    pub verbose: bool,
}

pub fn scan(paths: &[PathBuf], options: &ScanOptions) -> Result<ScanReport, String> {
    let mut roots = Vec::new();
    let mut projects = Vec::new();

    for path in paths {
        let root = path
            .canonicalize()
            .map_err(|err| format!("cannot scan {}: {err}", path.display()))?;
        roots.push(root.display().to_string());
        scan_dir(&root, &root, 0, options, &mut projects)?;
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

pub fn explain_path(path: &Path) -> Result<Explanation, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("{} has no valid file name", path.display()))?;

    let Some(mut draft) = rules::classify_candidate(parent, name, path.to_path_buf()) else {
        return Ok(Explanation {
            path: path.to_path_buf(),
            safety: Safety::Unknown,
            rule_id: None,
            category: None,
            reasons: vec!["no built-in rule matched this path".to_string()],
            warnings: Vec::new(),
            restore_hint: None,
        });
    };

    apply_path_safety(Path::new("."), &mut draft);

    Ok(Explanation {
        path: path.to_path_buf(),
        safety: draft.safety,
        rule_id: Some(draft.rule_id),
        category: Some(draft.category),
        reasons: draft.reasons,
        warnings: draft.warnings,
        restore_hint: Some(draft.restore_hint),
    })
}

fn scan_dir(
    dir: &Path,
    root: &Path,
    depth: usize,
    options: &ScanOptions,
    projects: &mut Vec<ProjectReport>,
) -> Result<(), String> {
    if depth > options.max_depth || is_skip_dir(dir) {
        return Ok(());
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries.flatten().collect::<Vec<_>>(),
        Err(err) => {
            if options.verbose {
                eprintln!("skip {}: {err}", dir.display());
            }
            return Ok(());
        }
    };

    let mut drafts = Vec::new();
    let mut child_dirs = Vec::new();

    for entry in entries {
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        let is_symlink = metadata.file_type().is_symlink();
        if !metadata.is_dir() && !is_symlink {
            continue;
        }

        let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };

        if rules::is_candidate_name(&name)
            && let Some(mut draft) = rules::classify_candidate(dir, &name, path.clone())
        {
            apply_path_safety(root, &mut draft);
            if should_include(&draft, options) {
                drafts.push(draft);
            }
            continue;
        }

        if metadata.is_dir() && !is_symlink && !is_skip_name(&name) {
            child_dirs.push(path);
        }
    }

    if !drafts.is_empty() {
        let project = build_project_report(dir, root, drafts, options)?;
        if !project.candidates.is_empty() {
            projects.push(project);
        }
    }

    for child in child_dirs {
        scan_dir(&child, root, depth + 1, options, projects)?;
    }

    Ok(())
}

fn build_project_report(
    dir: &Path,
    _root: &Path,
    drafts: Vec<CandidateDraft>,
    options: &ScanOptions,
) -> Result<ProjectReport, String> {
    let (kind, markers) = rules::detect_project_kind(dir);
    let git = git_info(dir);
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

    let mut candidates = Vec::new();
    for mut draft in drafts {
        if let Some(git) = &git
            && git.dirty
            && draft.safety == Safety::Safe
        {
            draft.safety = Safety::Caution;
            draft
                .warnings
                .push("project has uncommitted git changes".to_string());
        }

        let bytes = if draft.safety == Safety::Blocked {
            0
        } else {
            dir_size(&draft.path, options.verbose)
        };
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
    let candidate_paths = candidates
        .iter()
        .map(|candidate| PathBuf::from(&candidate.path))
        .collect::<Vec<_>>();
    let source_bytes =
        project_source_size(dir, &candidate_paths, options.max_depth, options.verbose);
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

    if is_runtime_or_system_path(&draft.path) {
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

fn dir_size(path: &Path, verbose: bool) -> u64 {
    let mut total: u64 = 0;
    for result in walkdir::WalkDir::new(path).follow_links(false) {
        let entry = match result {
            Ok(entry) => entry,
            Err(err) => {
                if verbose {
                    eprintln!("dir_size walk error under {}: {err}", path.display());
                }
                continue;
            }
        };
        match entry.metadata() {
            Ok(metadata) if metadata.is_file() => {
                total = total.saturating_add(metadata.len());
            }
            Ok(_) => {}
            Err(err) => {
                if verbose {
                    eprintln!(
                        "dir_size metadata error at {}: {err}",
                        entry.path().display()
                    );
                }
            }
        }
    }
    total
}

fn project_source_size(
    project_dir: &Path,
    candidate_paths: &[PathBuf],
    max_depth: usize,
    verbose: bool,
) -> u64 {
    let candidate_paths = candidate_paths.iter().cloned().collect::<HashSet<_>>();
    let mut total: u64 = 0;

    for result in walkdir::WalkDir::new(project_dir)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            let path = entry.path();
            if candidate_paths.contains(path) {
                return false;
            }
            entry
                .file_name()
                .to_str()
                .is_none_or(|name| !is_skip_name(name) && !rules::is_candidate_name(name))
        })
    {
        let entry = match result {
            Ok(entry) => entry,
            Err(err) => {
                if verbose {
                    eprintln!(
                        "project_source_size walk error under {}: {err}",
                        project_dir.display()
                    );
                }
                continue;
            }
        };
        match entry.metadata() {
            Ok(metadata) if metadata.is_file() => {
                total = total.saturating_add(metadata.len());
            }
            Ok(_) => {}
            Err(err) => {
                if verbose {
                    eprintln!(
                        "project_source_size metadata error at {}: {err}",
                        entry.path().display()
                    );
                }
            }
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

fn git_info(dir: &Path) -> Option<GitInfo> {
    let root = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !root.status.success() {
        return None;
    }
    let repo_root = String::from_utf8_lossy(&root.stdout).trim().to_string();
    if repo_root.is_empty() {
        return None;
    }

    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    let dirty = status.status.success() && !status.stdout.is_empty();

    Some(GitInfo { repo_root, dirty })
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
mod tests {
    use std::io::Write;

    use tempfile::TempDir;

    use super::*;

    fn options() -> ScanOptions {
        ScanOptions {
            max_depth: 6,
            min_size: 0,
            older_than: None,
            categories: None,
            rule_ids: None,
            include_blocked: true,
            verbose: false,
        }
    }

    #[test]
    fn detects_root_node_project() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("package.json"), "{}").unwrap();
        fs::create_dir(temp.path().join("node_modules")).unwrap();
        fs::write(temp.path().join("node_modules").join("x"), "abc").unwrap();

        let report = scan(&[temp.path().to_path_buf()], &options()).unwrap();

        assert_eq!(report.summary.candidates, 1);
        assert_eq!(
            report.projects[0].candidates[0].rule_id,
            "node.node_modules"
        );
        assert_eq!(report.projects[0].total_bytes, 3);
        assert_eq!(report.projects[0].project_bytes, 5);
        assert_eq!(report.projects[0].artifact_percent, 60.0);
    }

    #[test]
    fn blocks_plain_python_venv_without_marker() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("pyproject.toml"), "[project]\n").unwrap();
        fs::create_dir(temp.path().join("venv")).unwrap();

        let report = scan(&[temp.path().to_path_buf()], &options()).unwrap();

        assert_eq!(report.projects[0].candidates[0].safety, Safety::Blocked);
    }

    #[test]
    fn generic_build_without_marker_is_ignored() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("build")).unwrap();

        let report = scan(&[temp.path().to_path_buf()], &options()).unwrap();

        assert_eq!(report.summary.candidates, 0);
    }

    #[test]
    fn symlink_candidate_is_blocked() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("package.json"), "{}").unwrap();
        let real = temp.path().join("real_modules");
        fs::create_dir(&real).unwrap();
        let link = temp.path().join("node_modules");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&real, &link).unwrap();

        let report = scan(&[temp.path().to_path_buf()], &options()).unwrap();

        assert_eq!(report.projects[0].candidates[0].safety, Safety::Blocked);
    }

    #[test]
    fn detects_gradle_dart_dotnet_and_ruby_rules() {
        let temp = TempDir::new().unwrap();

        let gradle = temp.path().join("gradle");
        fs::create_dir(&gradle).unwrap();
        fs::write(gradle.join("build.gradle"), "plugins {}\n").unwrap();
        fs::create_dir(gradle.join("build")).unwrap();

        let dart = temp.path().join("dart");
        fs::create_dir(&dart).unwrap();
        fs::write(dart.join("pubspec.yaml"), "name: app\n").unwrap();
        fs::create_dir(dart.join(".dart_tool")).unwrap();

        let dotnet = temp.path().join("dotnet");
        fs::create_dir(&dotnet).unwrap();
        fs::write(dotnet.join("app.csproj"), "<Project />\n").unwrap();
        fs::create_dir(dotnet.join("bin")).unwrap();

        let ruby = temp.path().join("ruby");
        fs::create_dir_all(ruby.join("vendor").join("bundle")).unwrap();
        fs::write(ruby.join("Gemfile"), "source 'https://rubygems.org'\n").unwrap();

        let report = scan(&[temp.path().to_path_buf()], &options()).unwrap();
        let rule_ids = report
            .projects
            .iter()
            .flat_map(|project| project.candidates.iter())
            .map(|candidate| candidate.rule_id.as_str())
            .collect::<Vec<_>>();

        assert!(rule_ids.contains(&"java.gradle_build"));
        assert!(rule_ids.contains(&"dart.tool"));
        assert!(rule_ids.contains(&"dotnet.bin"));
        assert!(rule_ids.contains(&"ruby.vendor_bundle"));
    }

    #[test]
    fn dirty_git_marks_candidate_caution() {
        let temp = TempDir::new().unwrap();
        Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .arg("init")
            .output()
            .unwrap();
        fs::write(temp.path().join("package.json"), "{}").unwrap();
        fs::create_dir(temp.path().join("node_modules")).unwrap();
        let mut file = fs::File::create(temp.path().join("node_modules").join("x")).unwrap();
        writeln!(file, "abc").unwrap();

        let report = scan(&[temp.path().to_path_buf()], &options()).unwrap();

        assert_eq!(report.projects[0].candidates[0].safety, Safety::Caution);
    }

    #[test]
    fn risk_score_is_zero_when_no_markers_trip() {
        let temp = TempDir::new().unwrap();
        // Has lockfile, no git, fake old activity → no axis trips.
        fs::write(temp.path().join("Cargo.lock"), "[]").unwrap();
        let old = SystemTime::now() - Duration::from_secs(30 * 24 * 60 * 60);
        assert_eq!(compute_risk_score(None, old, temp.path()), 0.0);
    }

    #[test]
    fn risk_score_weights_match_spec() {
        let temp = TempDir::new().unwrap();
        // No lockfile → no_lockfile axis = 1.0, weight 0.20.
        let old = SystemTime::now() - Duration::from_secs(30 * 24 * 60 * 60);
        assert!((compute_risk_score(None, old, temp.path()) - 0.20).abs() < 1e-6);

        // Add a lockfile, use recent activity → recent_mtime axis = 1.0, weight 0.25.
        fs::write(temp.path().join("Cargo.lock"), "[]").unwrap();
        let recent = SystemTime::now();
        assert!((compute_risk_score(None, recent, temp.path()) - 0.25).abs() < 1e-6);

        // Add dirty git → dirty_git axis = 1.0, weight 0.40, total 0.65.
        let dirty = GitInfo {
            repo_root: temp.path().display().to_string(),
            dirty: true,
        };
        let score = compute_risk_score(Some(&dirty), recent, temp.path());
        assert!((score - 0.65).abs() < 1e-6, "expected 0.65, got {score}");
    }

    #[test]
    fn risk_score_max_is_0_85_until_root_boundary_axis_lands() {
        // No lockfile + recent activity + dirty git = 0.20 + 0.25 + 0.40 = 0.85.
        // The remaining 0.15 weight slot belongs to the deferred
        // root_boundary axis. When that axis lands (its own PR), this
        // test should flip to assert 1.0.
        let temp = TempDir::new().unwrap();
        let recent = SystemTime::now();
        let dirty = GitInfo {
            repo_root: temp.path().display().to_string(),
            dirty: true,
        };
        let score = compute_risk_score(Some(&dirty), recent, temp.path());
        assert!(score <= 1.0, "clamp invariant: score never exceeds 1.0");
        assert!(
            (score - 0.85).abs() < 1e-6,
            "current max is 0.85 (root_boundary deferred); got {score}"
        );
    }
}
