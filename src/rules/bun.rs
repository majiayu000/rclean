//! bun package manager global cache rule (cross-platform).
//!
//! Adds `bun.cache` for the install-time download cache at
//! `~/.bun/install/cache/`. bun maintains a single global cache
//! directory regardless of platform, so the rule is fully
//! cross-platform.
//!
//! Safe to delete; bun re-downloads packages on the next install.
//!
//! See `docs/specs/v0.3-developer-toolchain-extra.md` §3.1.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name == "cache" && parent_ends_with(project_dir, &[".bun", "install"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "bun.cache".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["bun package manager cache".to_string()],
            warnings: Vec::new(),
            restore_hint: "bun will repopulate the cache on the next install".to_string(),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_bun_install_cache() {
        let parent = PathBuf::from("/Users/me/.bun/install");
        let path = parent.join("cache");
        let draft = classify(&parent, "cache", &path).expect("should classify");
        assert_eq!(draft.rule_id, "bun.cache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.restore_hint.contains("bun"));
    }

    #[test]
    fn rejects_cache_outside_bun_install() {
        let parent = PathBuf::from("/Users/me/.cargo/registry");
        let path = parent.join("cache");
        assert!(classify(&parent, "cache", &path).is_none());
    }

    #[test]
    fn rejects_cache_under_bun_but_not_install() {
        let parent = PathBuf::from("/Users/me/.bun");
        let path = parent.join("cache");
        assert!(classify(&parent, "cache", &path).is_none());
    }

    #[test]
    fn rejects_other_names_inside_bun_install() {
        let parent = PathBuf::from("/Users/me/.bun/install");
        let path = parent.join("bin");
        assert!(classify(&parent, "bin", &path).is_none());
    }

    #[test]
    fn rejects_short_paths() {
        let parent = PathBuf::from("/install");
        let path = parent.join("cache");
        assert!(classify(&parent, "cache", &path).is_none());
    }
}
