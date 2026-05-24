//! Node/JS global cache rules (cross-platform).
//!
//! Phase 1 of the v0.2 "developer-grade mole" expansion. Adds three
//! rules for caches the JS package-manager toolchain maintains
//! outside individual projects:
//!
//! - `node.npm_cacache` — `<.npm>/_cacache/`. npm's
//!   content-addressable cache of downloaded tarballs. Safe to
//!   delete; npm rebuilds on the next install.
//! - `node.yarn_cache` — `Library/Caches/Yarn/` (macOS-style
//!   path). Yarn 1 classic cache. Safe to delete; Yarn rebuilds
//!   on the next install.
//! - `node.pnpm_store` — pnpm's user-level content-addressable
//!   store. Supports the legacy `~/.pnpm-store/vN` layout and the
//!   platform data-dir `pnpm/store` layout. Safe to delete; pnpm
//!   redownloads packages on the next install.
//!
//! Both are global-path rules — the path itself is the marker,
//! not a project marker like `package.json`. `apply_path_safety`
//! whitelists them via `rules::is_global_rule` so the generic
//! runtime-tree block doesn't demote them to Blocked.
//!
//! See `docs/specs/v0.2-developer-mole.md` §3.1.
//!
//! Not handled here (deferred to a later PR):
//! - Yarn 2+ (berry) per-project `.yarn/cache` — already a
//!   project-local concern, not global; will land in a separate
//!   `yarn.berry_cache` rule.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name == "_cacache" && parent_file_name_is(project_dir, ".npm") {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "node.npm_cacache".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["npm content-addressable cache".to_string()],
            warnings: Vec::new(),
            restore_hint: "npm will rebuild the cache on the next install".to_string(),
        });
    }

    if name == "Yarn" && parent_ends_with(project_dir, &["Library", "Caches"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "node.yarn_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["Yarn classic global cache".to_string()],
            warnings: Vec::new(),
            restore_hint: "Yarn will rebuild the cache on the next install".to_string(),
        });
    }

    if is_pnpm_store_version_name(name) && parent_file_name_is(project_dir, ".pnpm-store") {
        return Some(pnpm_store_draft(path, name));
    }

    if name == "store" && is_pnpm_store_parent(project_dir) {
        return Some(pnpm_store_draft(path, name));
    }

    None
}

pub(crate) fn is_pnpm_store_version_name(name: &str) -> bool {
    name.strip_prefix('v')
        .is_some_and(|version| !version.is_empty() && version.chars().all(|c| c.is_ascii_digit()))
}

fn parent_file_name_is(dir: &Path, expected: &str) -> bool {
    dir.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name == expected)
}

fn is_pnpm_store_parent(dir: &Path) -> bool {
    parent_ends_with(dir, &["Library", "pnpm"])
        || parent_ends_with(dir, &["Library", "Caches", "pnpm"])
        || parent_ends_with(dir, &[".local", "share", "pnpm"])
        || parent_ends_with(dir, &["AppData", "Local", "pnpm"])
}

fn pnpm_store_draft(path: &Path, name: &str) -> CandidateDraft {
    CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: "node.pnpm_store".to_string(),
        category: Category::Cache,
        safety: Safety::Safe,
        reasons: vec!["pnpm content-addressable store".to_string()],
        warnings: vec![
            "node_modules in existing projects use hardlinks into this store; \
             they will need a fresh `pnpm install` after deletion"
                .to_string(),
        ],
        restore_hint: "pnpm will rebuild the store on the next install".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_npm_cacache() {
        let parent = PathBuf::from("/Users/me/.npm");
        let path = parent.join("_cacache");
        let draft = classify(&parent, "_cacache", &path).expect("should classify");
        assert_eq!(draft.rule_id, "node.npm_cacache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.restore_hint.contains("npm"));
    }

    #[test]
    fn classifies_yarn_cache_under_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("Yarn");
        let draft = classify(&parent, "Yarn", &path).expect("should classify");
        assert_eq!(draft.rule_id, "node.yarn_cache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.restore_hint.contains("Yarn"));
    }

    #[test]
    fn classifies_legacy_pnpm_store_version_under_dot_pnpm_store() {
        let parent = PathBuf::from("/Users/me/.pnpm-store");
        let path = parent.join("v3");
        let Some(draft) = classify(&parent, "v3", &path) else {
            panic!("should classify");
        };
        assert_eq!(draft.rule_id, "node.pnpm_store");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.restore_hint.contains("pnpm"));
        assert!(
            draft.warnings.iter().any(|w| w.contains("hardlinks")),
            "pnpm_store draft should warn about hardlinks; got {:?}",
            draft.warnings
        );
    }

    #[test]
    fn classifies_platform_pnpm_store_under_library_pnpm() {
        let parent = PathBuf::from("/Users/me/Library/pnpm");
        let path = parent.join("store");
        let Some(draft) = classify(&parent, "store", &path) else {
            panic!("should classify");
        };
        assert_eq!(draft.rule_id, "node.pnpm_store");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
    }

    #[test]
    fn classifies_platform_pnpm_store_under_xdg_data_home() {
        let parent = PathBuf::from("/home/me/.local/share/pnpm");
        let path = parent.join("store");
        let Some(draft) = classify(&parent, "store", &path) else {
            panic!("should classify");
        };
        assert_eq!(draft.rule_id, "node.pnpm_store");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
    }

    #[test]
    fn rejects_cacache_outside_npm() {
        let parent = PathBuf::from("/Users/me/project");
        let path = parent.join("_cacache");
        assert!(classify(&parent, "_cacache", &path).is_none());
    }

    #[test]
    fn rejects_yarn_outside_library_caches() {
        let parent = PathBuf::from("/Users/me/somewhere");
        let path = parent.join("Yarn");
        assert!(classify(&parent, "Yarn", &path).is_none());
    }

    #[test]
    fn rejects_non_version_inside_dot_pnpm_store() {
        let parent = PathBuf::from("/Users/me/.pnpm-store");
        let path = parent.join("metadata");
        assert!(classify(&parent, "metadata", &path).is_none());
    }

    #[test]
    fn rejects_store_outside_canonical_pnpm_paths() {
        let parent = PathBuf::from("/Users/me/project/pnpm");
        let path = parent.join("store");
        assert!(classify(&parent, "store", &path).is_none());
    }

    #[test]
    fn rejects_other_names_inside_npm() {
        let parent = PathBuf::from("/Users/me/.npm");
        let path = parent.join("_logs");
        assert!(classify(&parent, "_logs", &path).is_none());
    }
}
