use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::cli::CleanArgs;
use crate::error::CleanError;
use crate::model::{Candidate, Safety, ScanReport, format_bytes};
use crate::scan::is_runtime_or_system_path;

#[derive(Debug, Clone, Copy)]
struct SelectableCandidate<'a> {
    project_path: &'a str,
    candidate: &'a Candidate,
}

#[derive(Debug, Clone)]
pub struct SelectedCandidate {
    pub path: PathBuf,
    pub bytes: u64,
    pub rule_id: String,
}

#[derive(Debug, Default)]
pub struct CleanResult {
    pub cleaned: Vec<SelectedCandidate>,
    pub failed: Vec<(SelectedCandidate, String)>,
}

pub fn select_candidates(
    report: &ScanReport,
    args: &CleanArgs,
) -> Result<Vec<SelectedCandidate>, CleanError> {
    let candidates = selectable_candidates(report);

    if !args.all {
        return select_interactively(&candidates, args.common.include_caution);
    }

    let mut selected = Vec::new();
    for item in candidates {
        if item.candidate.safety == Safety::Safe
            || (item.candidate.safety == Safety::Caution && args.common.include_caution)
        {
            selected.push(to_selected(item.candidate));
        }
    }
    Ok(selected)
}

fn selectable_candidates(report: &ScanReport) -> Vec<SelectableCandidate<'_>> {
    let mut candidates = Vec::new();
    for project in &report.projects {
        for candidate in &project.candidates {
            if candidate.safety != Safety::Blocked {
                candidates.push(SelectableCandidate {
                    project_path: &project.path,
                    candidate,
                });
            }
        }
    }
    candidates
}

fn select_interactively(
    candidates: &[SelectableCandidate<'_>],
    include_caution: bool,
) -> Result<Vec<SelectedCandidate>, CleanError> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    println!();
    println!("Select candidates to clean:");
    let mut current_project = "";
    for (index, item) in candidates.iter().enumerate() {
        let candidate = item.candidate;
        if item.project_path != current_project {
            current_project = item.project_path;
            println!();
            println!("Project: {current_project}");
        }
        let reason = candidate
            .reasons
            .first()
            .or_else(|| candidate.warnings.first())
            .map(String::as_str)
            .unwrap_or("-");
        println!(
            "  {:>2}. {:<8} {:<8} {:>10} {:<24} {}",
            index + 1,
            candidate.safety,
            candidate.category,
            format_bytes(candidate.bytes),
            candidate.name,
            reason
        );
    }
    println!("Enter numbers/ranges like 1,3,5 or 2-4. Use 'a' for all safe. Empty selects none.");
    print!("Selection: ");
    io::stdout()
        .flush()
        .map_err(|err| CleanError::Generic(format!("failed to flush stdout: {err}")))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| CleanError::Generic(format!("failed to read selection: {err}")))?;

    let input = input.trim();
    if input.eq_ignore_ascii_case("a") {
        return Ok(candidates
            .iter()
            .filter(|item| item.candidate.safety == Safety::Safe)
            .map(|item| to_selected(item.candidate))
            .collect());
    }

    let selected_indices = parse_selection(input, candidates.len())?;
    let mut selected = Vec::new();
    for index in selected_indices {
        let candidate = candidates[index].candidate;
        if candidate.safety == Safety::Safe
            || (candidate.safety == Safety::Caution && include_caution)
        {
            selected.push(to_selected(candidate));
        }
    }
    Ok(selected)
}

pub fn print_plan(selected: &[SelectedCandidate], permanent: bool, dry_run: bool) {
    if selected.is_empty() {
        println!();
        println!("Nothing selected.");
        return;
    }

    let total: u64 = selected.iter().map(|candidate| candidate.bytes).sum();
    println!();
    println!(
        "Plan: {} candidates, {} selected, mode: {}{}",
        selected.len(),
        format_bytes(total),
        if permanent { "permanent" } else { "trash" },
        if dry_run { " (dry run)" } else { "" }
    );
    for candidate in selected {
        println!(
            "  - {} ({}, {})",
            candidate.path.display(),
            candidate.rule_id,
            format_bytes(candidate.bytes)
        );
    }
}

pub fn confirm_if_needed(
    selected: &[SelectedCandidate],
    args: &CleanArgs,
) -> Result<(), CleanError> {
    if args.yes {
        return Ok(());
    }

    let total: u64 = selected.iter().map(|candidate| candidate.bytes).sum();
    let mode = if args.permanent {
        "permanently delete"
    } else {
        "move to Trash"
    };
    print!(
        "Confirm: {mode} {} candidates ({})? [y/N] ",
        selected.len(),
        format_bytes(total)
    );
    io::stdout()
        .flush()
        .map_err(|err| CleanError::Generic(format!("failed to flush stdout: {err}")))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| CleanError::Generic(format!("failed to read confirmation: {err}")))?;
    let answer = input.trim().to_ascii_lowercase();
    if answer == "y" || answer == "yes" {
        Ok(())
    } else {
        Err(CleanError::Generic("clean cancelled".to_string()))
    }
}

pub fn delete_selected(
    selected: &[SelectedCandidate],
    permanent: bool,
) -> Result<CleanResult, CleanError> {
    let mut result = CleanResult::default();

    for candidate in selected {
        if let Err(err) = validate_for_deletion(&candidate.path) {
            result.failed.push((candidate.clone(), err.to_string()));
            continue;
        }

        let outcome = if permanent {
            fs::remove_dir_all(&candidate.path).map_err(|err| err.to_string())
        } else {
            trash::delete(&candidate.path).map_err(|err| err.to_string())
        };

        match outcome {
            Ok(()) => result.cleaned.push(candidate.clone()),
            Err(err) => result.failed.push((candidate.clone(), err)),
        }
    }

    Ok(result)
}

pub fn check_broad_roots(roots: &[PathBuf]) -> Result<(), CleanError> {
    for root in roots {
        if let Some(canonical) = root.canonicalize().ok().or_else(|| Some(root.clone()))
            && is_broad_root(&canonical)
        {
            return Err(CleanError::Generic(format!(
                "refusing to clean against broad root {}: pass --allow-broad-root to override",
                canonical.display()
            )));
        }
    }
    Ok(())
}

fn is_broad_root(path: &Path) -> bool {
    if path.has_root()
        && !path
            .components()
            .any(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return true;
    }

    let broad: &[&str] = &[
        "/",
        "/etc",
        "/usr",
        "/var",
        "/opt",
        "/tmp",
        "/System",
        "/Library",
        "/private",
        "/Users",
        "/home",
        "/root",
        // macOS canonical forms (paths under /private)
        "/private/etc",
        "/private/var",
        "/private/tmp",
        "C:\\",
        "C:\\Windows",
        "C:\\Users",
        "C:\\Program Files",
        "C:\\Program Files (x86)",
    ];

    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from)
        && path == home
    {
        return true;
    }
    if let Some(userprofile) = std::env::var_os("USERPROFILE").map(PathBuf::from)
        && path == userprofile
    {
        return true;
    }
    let path_str = path.to_string_lossy();
    broad.iter().any(|b| path_str.as_ref() == *b)
}

pub(crate) fn validate_for_deletion(path: &Path) -> Result<(), CleanError> {
    let metadata = fs::symlink_metadata(path).map_err(|err| {
        CleanError::Generic(format!(
            "{} no longer exists or cannot be read: {err}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(CleanError::Generic(format!(
            "refusing to delete {}: path is now a symlink",
            path.display()
        )));
    }
    if !metadata.is_dir() {
        return Err(CleanError::Generic(format!(
            "refusing to delete {}: path is no longer a directory",
            path.display()
        )));
    }

    let canonical = path.canonicalize().map_err(|err| {
        CleanError::Generic(format!("failed to canonicalize {}: {err}", path.display()))
    })?;
    if is_runtime_or_system_path(&canonical) {
        return Err(CleanError::Generic(format!(
            "refusing to delete {}: resolves to a protected runtime or system path",
            path.display()
        )));
    }

    Ok(())
}

pub fn print_clean_result(result: &CleanResult) {
    let total: u64 = result.cleaned.iter().map(|candidate| candidate.bytes).sum();
    println!();
    println!(
        "Cleaned: {} candidates, {}",
        result.cleaned.len(),
        format_bytes(total)
    );
    if !result.failed.is_empty() {
        println!("Failed: {}", result.failed.len());
        for (candidate, error) in &result.failed {
            println!("  - {}: {}", candidate.path.display(), error);
        }
    }
}

pub fn parse_selection(input: &str, count: usize) -> Result<Vec<usize>, CleanError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.eq_ignore_ascii_case("a") {
        return Ok((0..count).collect());
    }

    let mut selected = Vec::new();
    for part in trimmed
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if let Some((start, end)) = part.split_once('-') {
            let start = parse_selection_number(start, count)?;
            let end = parse_selection_number(end, count)?;
            if start > end {
                return Err(CleanError::Generic(format!("invalid range '{part}'")));
            }
            for index in start..=end {
                if !selected.contains(&index) {
                    selected.push(index);
                }
            }
        } else {
            let index = parse_selection_number(part, count)?;
            if !selected.contains(&index) {
                selected.push(index);
            }
        }
    }
    Ok(selected)
}

fn parse_selection_number(raw: &str, count: usize) -> Result<usize, CleanError> {
    let number = raw
        .trim()
        .parse::<usize>()
        .map_err(|_| CleanError::Generic(format!("invalid selection '{raw}'")))?;
    if number == 0 || number > count {
        return Err(CleanError::Generic(format!(
            "selection {number} is out of range 1-{count}"
        )));
    }
    Ok(number - 1)
}

fn to_selected(candidate: &Candidate) -> SelectedCandidate {
    SelectedCandidate {
        path: PathBuf::from(&candidate.path),
        bytes: candidate.bytes,
        rule_id: candidate.rule_id.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parses_interactive_selection() {
        assert_eq!(parse_selection("", 5).unwrap(), Vec::<usize>::new());
        assert_eq!(parse_selection("a", 3).unwrap(), vec![0, 1, 2]);
        assert_eq!(parse_selection("1,3-4,3", 5).unwrap(), vec![0, 2, 3]);
        assert!(parse_selection("0", 3).is_err());
        assert!(parse_selection("4", 3).is_err());
        assert!(parse_selection("3-1", 3).is_err());
    }

    #[test]
    fn check_broad_roots_rejects_root_slash() {
        let err = check_broad_roots(&[PathBuf::from("/")])
            .expect_err("/ must be rejected as broad")
            .to_string();
        assert!(err.contains("broad root"), "unexpected error: {err}");
    }

    #[test]
    fn check_broad_roots_rejects_etc() {
        let err = check_broad_roots(&[PathBuf::from("/etc")])
            .expect_err("/etc must be rejected as broad")
            .to_string();
        assert!(err.contains("broad root"), "unexpected error: {err}");
    }

    #[test]
    fn check_broad_roots_accepts_normal_project_path() {
        let temp = TempDir::new().unwrap();
        check_broad_roots(&[temp.path().to_path_buf()])
            .expect("a normal tempdir path must not be flagged as broad");
    }

    #[test]
    fn validate_accepts_real_directory() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("artifact");
        fs::create_dir(&dir).unwrap();
        validate_for_deletion(&dir).expect("real directory must validate");
    }

    #[test]
    fn validate_rejects_symlink() {
        let temp = TempDir::new().unwrap();
        let real = temp.path().join("real");
        let link = temp.path().join("link");
        fs::create_dir(&real).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&real, &link).unwrap();
        let err = validate_for_deletion(&link)
            .expect_err("symlink must be rejected")
            .to_string();
        assert!(err.contains("symlink"), "unexpected error: {err}");
    }

    #[test]
    fn validate_rejects_missing_path() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("missing");
        let err = validate_for_deletion(&missing)
            .expect_err("missing path must be rejected")
            .to_string();
        assert!(
            err.contains("no longer exists") || err.contains("cannot be read"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validate_rejects_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("file");
        fs::write(&file, b"x").unwrap();
        let err = validate_for_deletion(&file)
            .expect_err("file must be rejected")
            .to_string();
        assert!(
            err.contains("no longer a directory"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn delete_selected_skips_swapped_symlink_target() {
        let temp = TempDir::new().unwrap();
        let real = temp.path().join("real");
        let candidate_path = temp.path().join("artifact");
        fs::create_dir(&real).unwrap();
        fs::create_dir(&candidate_path).unwrap();

        let selected = vec![SelectedCandidate {
            path: candidate_path.clone(),
            bytes: 0,
            rule_id: "test".to_string(),
        }];

        // TOCTOU: replace the candidate directory with a symlink between scan and delete.
        fs::remove_dir(&candidate_path).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, &candidate_path).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&real, &candidate_path).unwrap();

        let result = delete_selected(&selected, true).unwrap();
        assert!(result.cleaned.is_empty());
        assert_eq!(result.failed.len(), 1);
        assert!(real.is_dir(), "symlink target must not be deleted");
    }
}
