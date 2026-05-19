//! Node/JS global cache rules (cross-platform).
//!
//! Phase 1 of the v0.2 "developer-grade mole" expansion. Adds two
//! rules for caches the JS package-manager toolchain maintains
//! outside individual projects:
//!
//! - `node.npm_cacache` — `<.npm>/_cacache/`. npm's
//!   content-addressable cache of downloaded tarballs. Safe to
//!   delete; npm rebuilds on the next install.
//! - `node.yarn_cache` — `Library/Caches/Yarn/` (macOS-style
//!   path). Yarn 1 classic cache. Safe to delete; Yarn rebuilds
//!   on the next install.
//!
//! Both are global-path rules — the path itself is the marker,
//! not a project marker like `package.json`. `apply_path_safety`
//! whitelists them via `rules::is_global_rule` so the generic
//! runtime-tree block doesn't demote them to Blocked.
//!
//! See `docs/specs/v0.2-developer-mole.md` §3.1.
//!
//! Not handled here (deferred to a later PR):
//! - `~/.pnpm-store` — sits directly under `$HOME`, so it cannot
//!   be classified through the existing parent-anchored dispatch.
//!   Needs path-anchored dispatch first.
//! - Yarn 2+ (berry) per-project `.yarn/cache` — already a
//!   project-local concern, not global; will land in a separate
//!   `yarn.berry_cache` rule.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};

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

    None
}

fn parent_file_name_is(dir: &Path, expected: &str) -> bool {
    dir.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name == expected)
}

fn parent_ends_with(dir: &Path, suffix: &[&str]) -> bool {
    let components: Vec<&str> = dir
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    if components.len() < suffix.len() {
        return false;
    }
    let tail = &components[components.len() - suffix.len()..];
    tail == suffix
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
    fn rejects_other_names_inside_npm() {
        let parent = PathBuf::from("/Users/me/.npm");
        let path = parent.join("_logs");
        assert!(classify(&parent, "_logs", &path).is_none());
    }
}
