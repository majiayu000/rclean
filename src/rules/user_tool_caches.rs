//! Conservative user-level tool and app cache rules.
//!
//! These rules cover the #116 backlog items that have exact user-level
//! anchors and clear restore paths. They deliberately avoid broad
//! `Application Support` or container cleanup.

use std::cmp::Ordering;
use std::fs;
use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name == "compact_index" && parent_ends_with(project_dir, &[".bundle", "cache"]) {
        return Some(safe_cache(
            path,
            name,
            "ruby.bundle_compact_index",
            "Bundler compact index cache",
            "Bundler will refetch the compact index on the next install",
        ));
    }

    if name == "cache" && parent_ends_with(project_dir, &[".kube"]) {
        return Some(safe_cache(
            path,
            name,
            "cloud.kube_cache",
            "Kubernetes CLI discovery cache",
            "kubectl will recreate the discovery cache on the next use",
        ));
    }

    if name == "logs" && parent_ends_with(project_dir, &[".config", "gcloud"]) {
        return Some(safe_cache(
            path,
            name,
            "cloud.gcloud_logs",
            "gcloud CLI log cache",
            "gcloud will recreate logs on the next run",
        ));
    }

    if let Some(rule_id) = editor_cache_rule_id(project_dir)
        && is_editor_cache_name(name)
    {
        return Some(app_cache(
            path,
            name,
            rule_id,
            "Editor rebuildable cache/log directory",
            "Close the editor first; it will recreate this cache on the next launch",
        ));
    }

    if is_obsolete_extension(project_dir, name) {
        let rule_id = if parent_ends_with(project_dir, &[".vscode", "extensions"]) {
            "editor.vscode_obsolete_extension"
        } else {
            "editor.cursor_obsolete_extension"
        };
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: rule_id.to_string(),
            category: Category::Deps,
            safety: Safety::Caution,
            reasons: vec!["Older editor extension version with a newer sibling installed".to_string()],
            warnings: vec![
                "Only remove obsolete extension versions when the editor is closed; extensions can be reinstalled from the marketplace"
                    .to_string(),
            ],
            restore_hint: "Reinstall the extension from the marketplace if needed".to_string(),
        });
    }

    if parent_ends_with(project_dir, &[".local", "share", "claude", "versions"])
        && is_obsolete_version_dir(project_dir, name)
    {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "claude.old_version".to_string(),
            category: Category::Deps,
            safety: Safety::Caution,
            reasons: vec!["Older Claude Code version with a newer sibling installed".to_string()],
            warnings: vec![
                "Keep the newest version and do not touch Claude sessions, history, or project state"
                    .to_string(),
            ],
            restore_hint: "Claude Code can reinstall the version if it is needed again".to_string(),
        });
    }

    if is_known_electron_support_parent(project_dir) && is_editor_cache_name(name) {
        return Some(app_cache(
            path,
            name,
            "app.electron_cache",
            "Known Electron app rebuildable cache/log directory",
            "Close the app first; it will recreate this cache on the next launch",
        ));
    }

    None
}

pub(crate) fn is_dynamic_candidate_name(name: &str) -> bool {
    is_dawn_cache_name(name)
        || split_versioned_name(name).is_some()
        || parse_dotted_version(name).is_some()
}

fn safe_cache(
    path: &Path,
    name: &str,
    rule_id: &str,
    reason: &str,
    restore_hint: &str,
) -> CandidateDraft {
    CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: rule_id.to_string(),
        category: Category::Cache,
        safety: Safety::Safe,
        reasons: vec![reason.to_string()],
        warnings: Vec::new(),
        restore_hint: restore_hint.to_string(),
    }
}

fn app_cache(
    path: &Path,
    name: &str,
    rule_id: &str,
    reason: &str,
    restore_hint: &str,
) -> CandidateDraft {
    CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: rule_id.to_string(),
        category: Category::Cache,
        safety: Safety::Caution,
        reasons: vec![reason.to_string()],
        warnings: vec!["Skip this candidate while the owning app is running".to_string()],
        restore_hint: restore_hint.to_string(),
    }
}

fn editor_cache_rule_id(parent: &Path) -> Option<&'static str> {
    if parent_ends_with(parent, &["Library", "Application Support", "Code"]) {
        Some("editor.vscode_cache")
    } else if parent_ends_with(parent, &["Library", "Application Support", "Cursor"]) {
        Some("editor.cursor_cache")
    } else {
        None
    }
}

fn is_editor_cache_name(name: &str) -> bool {
    matches!(
        name,
        "logs" | "Cache" | "CachedData" | "Code Cache" | "GPUCache"
    ) || is_dawn_cache_name(name)
}

fn is_dawn_cache_name(name: &str) -> bool {
    name.starts_with("Dawn") && name.ends_with("Cache") && name.len() > "DawnCache".len()
}

fn is_known_electron_support_parent(parent: &Path) -> bool {
    parent_ends_with(parent, &["Library", "Application Support", "Notion"])
        || parent_ends_with(parent, &["Library", "Application Support", "Slack"])
        || parent_ends_with(
            parent,
            &["Library", "Application Support", "LarkInternational"],
        )
}

fn is_obsolete_extension(parent: &Path, name: &str) -> bool {
    let Some((extension_id, version)) = split_versioned_name(name) else {
        return false;
    };
    if !parent_ends_with(parent, &[".vscode", "extensions"])
        && !parent_ends_with(parent, &[".cursor", "extensions"])
    {
        return false;
    }
    has_newer_sibling(parent, extension_id, &version)
}

fn is_obsolete_version_dir(parent: &Path, name: &str) -> bool {
    let Some(version) = parse_dotted_version(name) else {
        return false;
    };
    has_newer_sibling(parent, "", &version)
}

fn has_newer_sibling(parent: &Path, id: &str, current: &[u64]) -> bool {
    let Ok(entries) = fs::read_dir(parent) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            return false;
        };
        let candidate_version = if id.is_empty() {
            parse_dotted_version(&name)
        } else {
            split_versioned_name(&name)
                .and_then(|(candidate_id, version)| (candidate_id == id).then_some(version))
        };
        candidate_version.is_some_and(|version| compare_versions(&version, current).is_gt())
    })
}

fn split_versioned_name(name: &str) -> Option<(&str, Vec<u64>)> {
    let (id, version) = name.rsplit_once('-')?;
    if !id.contains('.') {
        return None;
    }
    Some((id, parse_dotted_version(version)?))
}

fn parse_dotted_version(version: &str) -> Option<Vec<u64>> {
    if version.is_empty() || !version.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
        return None;
    }
    let parts: Option<Vec<u64>> = version
        .split('.')
        .map(|part| {
            (!part.is_empty())
                .then(|| part.parse::<u64>().ok())
                .flatten()
        })
        .collect();
    parts.filter(|parts| !parts.is_empty())
}

fn compare_versions(a: &[u64], b: &[u64]) -> Ordering {
    let max_len = a.len().max(b.len());
    for index in 0..max_len {
        let left = a.get(index).copied().unwrap_or(0);
        let right = b.get(index).copied().unwrap_or(0);
        match left.cmp(&right) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn classifies_bundler_compact_index() {
        let parent = PathBuf::from("/Users/me/.bundle/cache");
        let path = parent.join("compact_index");
        let draft = classify(&parent, "compact_index", &path).expect("should classify");
        assert_eq!(draft.rule_id, "ruby.bundle_compact_index");
        assert_eq!(draft.safety, Safety::Safe);
    }

    #[test]
    fn classifies_kube_cache_without_touching_config() {
        let parent = PathBuf::from("/Users/me/.kube");
        let path = parent.join("cache");
        let draft = classify(&parent, "cache", &path).expect("should classify");
        assert_eq!(draft.rule_id, "cloud.kube_cache");
        assert_eq!(draft.safety, Safety::Safe);
        assert!(classify(&parent, "config", &parent.join("config")).is_none());
    }

    #[test]
    fn classifies_gcloud_logs_without_touching_credentials() {
        let parent = PathBuf::from("/Users/me/.config/gcloud");
        let path = parent.join("logs");
        let draft = classify(&parent, "logs", &path).expect("should classify");
        assert_eq!(draft.rule_id, "cloud.gcloud_logs");
        assert_eq!(draft.safety, Safety::Safe);
        assert!(classify(&parent, "credentials.db", &parent.join("credentials.db")).is_none());
    }

    #[test]
    fn classifies_vscode_and_cursor_exact_cache_subdirs() {
        for (parent, rule_id) in [
            (
                "/Users/me/Library/Application Support/Code",
                "editor.vscode_cache",
            ),
            (
                "/Users/me/Library/Application Support/Cursor",
                "editor.cursor_cache",
            ),
        ] {
            let parent = PathBuf::from(parent);
            for name in ["logs", "Cache", "CachedData", "Code Cache", "GPUCache"] {
                let draft = classify(&parent, name, &parent.join(name)).expect("should classify");
                assert_eq!(draft.rule_id, rule_id);
                assert_eq!(draft.safety, Safety::Caution);
            }
            assert!(classify(&parent, "User", &parent.join("User")).is_none());
            assert!(classify(&parent, "globalStorage", &parent.join("globalStorage")).is_none());
            assert!(
                classify(
                    &parent,
                    "workspaceStorage",
                    &parent.join("workspaceStorage")
                )
                .is_none()
            );
        }
    }

    #[test]
    fn classifies_obsolete_vscode_extension_only_when_newer_sibling_exists() {
        let temp = TempDir::new().unwrap();
        let parent = temp.path().join(".vscode").join("extensions");
        fs::create_dir_all(parent.join("publisher.tool-1.2.0")).unwrap();
        fs::create_dir_all(parent.join("publisher.tool-1.3.0")).unwrap();

        let old = parent.join("publisher.tool-1.2.0");
        let new = parent.join("publisher.tool-1.3.0");
        let draft = classify(&parent, "publisher.tool-1.2.0", &old).expect("should classify");
        assert_eq!(draft.rule_id, "editor.vscode_obsolete_extension");
        assert_eq!(draft.safety, Safety::Caution);
        assert!(classify(&parent, "publisher.tool-1.3.0", &new).is_none());
    }

    #[test]
    fn classifies_obsolete_claude_version_only_when_newer_sibling_exists() {
        let temp = TempDir::new().unwrap();
        let parent = temp.path().join(".local/share/claude/versions");
        fs::create_dir_all(parent.join("1.0.0")).unwrap();
        fs::create_dir_all(parent.join("1.1.0")).unwrap();

        let old = parent.join("1.0.0");
        let draft = classify(&parent, "1.0.0", &old).expect("should classify");
        assert_eq!(draft.rule_id, "claude.old_version");
        assert_eq!(draft.safety, Safety::Caution);
        assert!(classify(&parent, "1.1.0", &parent.join("1.1.0")).is_none());
    }

    #[test]
    fn classifies_known_electron_cache_but_not_app_state() {
        let parent = PathBuf::from("/Users/me/Library/Application Support/Notion");
        let draft =
            classify(&parent, "GPUCache", &parent.join("GPUCache")).expect("should classify");
        assert_eq!(draft.rule_id, "app.electron_cache");
        assert_eq!(draft.safety, Safety::Caution);
        assert!(classify(&parent, "Partitions", &parent.join("Partitions")).is_none());
    }
}
