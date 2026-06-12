//! Per-candidate safety promotion and pruning predicates.
//!
//! These are the trust-boundary checks the walker and `explain`
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

use std::fs;
use std::fs::Metadata;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::Path;

use crate::model::{CandidateDraft, Safety};
use crate::path_util::path_file_name;
use crate::rules;

const RUNTIME_OR_SYSTEM_COMPONENTS: &[&str] = &[
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
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DangerousLink {
    Symlink,
    #[cfg(unix)]
    HardlinkedFile,
    #[cfg(windows)]
    ReparsePoint,
}

impl DangerousLink {
    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::Symlink => "symlink",
            #[cfg(unix)]
            Self::HardlinkedFile => "hardlinked file",
            #[cfg(windows)]
            Self::ReparsePoint => "junction or reparse point",
        }
    }
}

pub(crate) fn dangerous_link_kind(metadata: &Metadata) -> Option<DangerousLink> {
    if metadata.file_type().is_symlink() {
        return Some(DangerousLink::Symlink);
    }

    #[cfg(unix)]
    {
        // Directories normally have nlink > 1 on Unix, so only regular
        // hardlinked files are dangerous here.
        if metadata.file_type().is_file() && metadata.nlink() > 1 {
            return Some(DangerousLink::HardlinkedFile);
        }
    }

    #[cfg(windows)]
    {
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Some(DangerousLink::ReparsePoint);
        }
    }

    None
}

pub(crate) fn apply_path_safety(root: &Path, draft: &mut CandidateDraft) {
    let metadata = fs::symlink_metadata(&draft.path);
    if let Ok(metadata) = metadata.as_ref()
        && let Some(kind) = dangerous_link_kind(metadata)
    {
        draft.safety = Safety::Blocked;
        draft
            .warnings
            .push(format!("candidate is a {}", kind.description()));
        return;
    }

    if is_protected_user_data_path(&draft.path)
        && !rules::allows_protected_user_data_path(&draft.rule_id)
    {
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
    path_file_name(path).is_some_and(is_skip_name)
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
    path.components()
        .filter_map(component_name)
        .any(|name| RUNTIME_OR_SYSTEM_COMPONENTS.contains(&name))
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
            (Some("Library"), "Containers") => return true,
            (Some("Chrome"), "Default") => return true,
            (Some("Code") | Some("Cursor"), "User" | "globalStorage" | "workspaceStorage") => {
                return true;
            }
            (Some("Notion"), "Partitions") => return true,
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
    use super::{
        DangerousLink, dangerous_link_kind, is_protected_user_data_path, is_runtime_or_system_path,
    };
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn regular_directory_is_not_a_dangerous_link() {
        let temp = TempDir::new().unwrap();
        let metadata = fs::symlink_metadata(temp.path()).unwrap();

        assert_eq!(dangerous_link_kind(&metadata), None);
    }

    #[test]
    #[cfg(unix)]
    fn detects_hardlinked_regular_file() {
        let temp = TempDir::new().unwrap();
        let original = temp.path().join("original");
        let hardlink = temp.path().join("hardlink");
        fs::write(&original, "content").unwrap();
        fs::hard_link(&original, &hardlink).unwrap();

        let metadata = fs::symlink_metadata(&hardlink).unwrap();

        assert_eq!(
            dangerous_link_kind(&metadata),
            Some(DangerousLink::HardlinkedFile)
        );
    }

    #[test]
    #[cfg(windows)]
    fn detects_junction_as_dangerous_link() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target");
        let junction = temp.path().join("junction");
        fs::create_dir(&target).unwrap();
        let output = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&junction)
            .arg(&target)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "mklink failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let metadata = fs::symlink_metadata(&junction).unwrap();

        let kind = dangerous_link_kind(&metadata).expect("junction must be dangerous");
        assert!(
            matches!(kind, DangerousLink::Symlink | DangerousLink::ReparsePoint),
            "unexpected dangerous link kind: {kind:?}"
        );
    }

    #[test]
    fn runtime_or_system_path_matches_any_protected_component() {
        for path in [
            "/Users/me/.cargo/registry/cache",
            "/Users/me/Library/Caches/example",
            "/Users/me/project/.Trash/old-file",
        ] {
            assert!(
                is_runtime_or_system_path(&PathBuf::from(path)),
                "expected {path} to be protected"
            );
        }
    }

    #[test]
    fn runtime_or_system_path_requires_exact_component_match() {
        for path in [
            "/Users/me/cargo/registry/cache",
            "/Users/me/MyLibrary/Caches/example",
            "/Users/me/project/Trash/old-file",
        ] {
            assert!(
                !is_runtime_or_system_path(&PathBuf::from(path)),
                "did not expect {path} to be protected"
            );
        }
    }

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
    fn protects_macos_app_container_and_profile_state() {
        for path in [
            "/Users/me/Library/Containers/com.tencent.xinWeChat",
            "/Users/me/Library/Application Support/Google/Chrome/Default",
            "/Users/me/Library/Application Support/Code/User",
            "/Users/me/Library/Application Support/Code/globalStorage",
            "/Users/me/Library/Application Support/Cursor/workspaceStorage",
            "/Users/me/Library/Application Support/Notion/Partitions",
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
