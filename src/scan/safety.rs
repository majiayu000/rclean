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
        if previous == Some(".codex") && matches!(name, "sessions" | "memories") {
            return true;
        }
        previous = Some(name);
    }
    false
}

fn component_name(component: std::path::Component<'_>) -> Option<&str> {
    component.as_os_str().to_str()
}
