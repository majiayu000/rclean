//! Per-candidate safety promotion and pruning predicates.
//!
//! These are the trust-boundary checks the walker and `explain_path`
//! both share:
//!
//! - [`apply_path_safety`]: symlinks → Blocked; system/runtime
//!   directories → Blocked; candidates resolving outside the scan
//!   root → Blocked.
//! - [`is_skip_dir`] / [`is_skip_name`]: directories we never descend
//!   into (`.git`, toolchain caches, the Trash, the macOS Library).
//! - [`is_protected_user_data_path`]: user-owned records that are never
//!   cleanable, even if a rule or plan points at them.
//! - [`is_runtime_or_system_path`]: same allowlist applied as an
//!   any-component check on the candidate path.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::model::{CandidateDraft, Safety};
use crate::rules;

pub(crate) fn apply_path_safety(root: &Path, draft: &mut CandidateDraft) {
    let metadata = fs::symlink_metadata(&draft.path);
    if metadata
        .as_ref()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        draft.safety = Safety::Blocked;
        draft.warnings.push("candidate is a symlink".to_string());
        return;
    }

    if is_protected_user_data_path(&draft.path) {
        draft.safety = Safety::Blocked;
        draft
            .warnings
            .push("candidate is inside protected user data".to_string());
        return;
    }

    // Global rules target paths that live inside the user's Library /
    // runtime tree by design. Their classifier already establishes that
    // the path is a rebuildable cache, so the generic runtime/system-path
    // block would otherwise hide them.
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

pub(crate) fn is_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_skip_name)
}

pub(crate) fn is_skip_name(name: &str) -> bool {
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

pub(crate) fn is_protected_user_data_path(path: &Path) -> bool {
    let mut previous = None;
    for name in path.components().filter_map(component_name) {
        match (previous, name) {
            (Some(".codex"), "sessions" | "memories") => return true,
            (
                Some(".claude"),
                "projects" | "sessions" | "history.jsonl" | "shell-snapshots" | "file-history"
                | "todos",
            ) => return true,
            _ => {}
        }
        previous = Some(name);
    }
    false
}

fn component_name(component: std::path::Component<'_>) -> Option<&str> {
    component.as_os_str().to_str()
}

#[cfg(test)]
mod tests {
    use super::is_protected_user_data_path;
    use std::path::PathBuf;

    #[test]
    fn protects_codex_user_records() {
        assert!(is_protected_user_data_path(&PathBuf::from(
            "/Users/me/.codex/sessions"
        )));
        assert!(is_protected_user_data_path(&PathBuf::from(
            "/Users/me/.codex/memories/note.md"
        )));
    }

    #[test]
    fn protects_claude_code_user_records() {
        for path in [
            "/Users/me/.claude/projects/-some-encoded-path/abc.jsonl",
            "/Users/me/.claude/sessions/2026-05-24",
            "/Users/me/.claude/history.jsonl",
            "/Users/me/.claude/shell-snapshots/snap-1.json",
            "/Users/me/.claude/file-history/foo.diff",
            "/Users/me/.claude/todos/today.md",
        ] {
            assert!(
                is_protected_user_data_path(&PathBuf::from(path)),
                "expected {path} to be protected"
            );
        }
    }

    #[test]
    fn does_not_protect_rebuildable_claude_subdirs() {
        for path in [
            "/Users/me/.claude/cache/x",
            "/Users/me/.claude/paste-cache/y",
            "/Users/me/.claude/downloads/z",
        ] {
            assert!(
                !is_protected_user_data_path(&PathBuf::from(path)),
                "did not expect {path} to be protected"
            );
        }
    }

    #[test]
    fn does_not_protect_unrelated_paths() {
        assert!(!is_protected_user_data_path(&PathBuf::from(
            "/Users/me/work/projects/foo"
        )));
        assert!(!is_protected_user_data_path(&PathBuf::from(
            "/Users/me/.config/sessions/foo"
        )));
    }
}
