//! pip global cache rule (cross-platform).
//!
//! Phase 1 of the v0.2 "developer-grade mole" expansion. Adds one
//! rule for pip's user-level wheel cache:
//!
//! - `pip.cache` — the directory pip uses to cache downloaded
//!   wheels and HTTP responses. Two canonical layouts:
//!     - macOS:   `~/Library/Caches/pip`
//!     - Linux:   `~/.cache/pip`
//!
//!   Safe to delete; pip recreates and repopulates on the next
//!   install.
//!
//! This is a global-path rule. `apply_path_safety` whitelists it
//! via `rules::is_global_rule` so the generic runtime-tree block
//! (`Library`) doesn't demote it on macOS.
//!
//! See `docs/specs/v0.2-developer-mole.md` §3.1.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name != "pip" {
        return None;
    }

    // macOS: ~/Library/Caches/pip
    let mac_match = parent_ends_with(project_dir, &["Library", "Caches"]);
    // Linux / XDG default: ~/.cache/pip
    let xdg_match = parent_ends_with(project_dir, &[".cache"]);

    if !(mac_match || xdg_match) {
        return None;
    }

    Some(CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: "pip.cache".to_string(),
        category: Category::Cache,
        safety: Safety::Safe,
        reasons: vec!["pip wheel and HTTP response cache".to_string()],
        warnings: Vec::new(),
        restore_hint: "pip will repopulate the cache on the next install".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_pip_under_macos_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("pip");
        let draft = classify(&parent, "pip", &path).expect("should classify");
        assert_eq!(draft.rule_id, "pip.cache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.restore_hint.contains("pip"));
    }

    #[test]
    fn classifies_pip_under_xdg_cache() {
        let parent = PathBuf::from("/home/me/.cache");
        let path = parent.join("pip");
        let draft = classify(&parent, "pip", &path).expect("should classify");
        assert_eq!(draft.rule_id, "pip.cache");
    }

    #[test]
    fn rejects_pip_outside_canonical_paths() {
        let parent = PathBuf::from("/Users/me/project");
        let path = parent.join("pip");
        assert!(classify(&parent, "pip", &path).is_none());
    }

    #[test]
    fn rejects_other_names_inside_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("Spotify");
        assert!(classify(&parent, "Spotify", &path).is_none());
    }
}
