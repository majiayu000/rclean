//! pre-commit hook framework cache rule (cross-platform).
//!
//! Adds `pre_commit.cache` for `~/.cache/pre-commit/`, where the
//! pre-commit framework clones hook repositories and provisions
//! their language environments. pre-commit does *not* follow the
//! macOS `~/Library/Caches` convention — it hardcodes
//! `$XDG_CACHE_HOME/pre-commit` (default `~/.cache/pre-commit`)
//! on every platform.
//!
//! Safe to delete; `pre-commit run` reinitializes hooks on the
//! next invocation.
//!
//! See `docs/specs/v0.3-developer-toolchain-extra.md` §3.1.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name == "pre-commit" && parent_ends_with(project_dir, &[".cache"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "pre_commit.cache".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["pre-commit framework hook cache".to_string()],
            warnings: Vec::new(),
            restore_hint: "pre-commit will reinitialize hooks on the next run".to_string(),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_pre_commit_under_dot_cache() {
        let parent = PathBuf::from("/Users/me/.cache");
        let path = parent.join("pre-commit");
        let draft = classify(&parent, "pre-commit", &path).expect("should classify");
        assert_eq!(draft.rule_id, "pre_commit.cache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.restore_hint.contains("pre-commit"));
    }

    #[test]
    fn rejects_pre_commit_outside_dot_cache() {
        let parent = PathBuf::from("/Users/me/projects/foo");
        let path = parent.join("pre-commit");
        assert!(classify(&parent, "pre-commit", &path).is_none());
    }

    #[test]
    fn rejects_other_names_inside_dot_cache() {
        let parent = PathBuf::from("/Users/me/.cache");
        let path = parent.join("pip");
        assert!(classify(&parent, "pip", &path).is_none());
    }

    #[test]
    fn rejects_short_paths() {
        let parent = PathBuf::from("/.cache");
        let path = parent.join("pre-commit");
        // Tail matches [".cache"] so this *should* fire — keep as a
        // sanity check that parent_ends_with works at minimum depth.
        let draft = classify(&parent, "pre-commit", &path);
        assert!(draft.is_some());
    }

    #[test]
    fn rejects_root_only_path() {
        let parent = PathBuf::from("/");
        let path = parent.join("pre-commit");
        assert!(classify(&parent, "pre-commit", &path).is_none());
    }
}
