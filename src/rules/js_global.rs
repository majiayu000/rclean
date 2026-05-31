//! Deno global cache rule (cross-platform).
//!
//! Issue #103 extension to the v0.2 "developer-grade mole" expansion.
//! Adds coverage for the cache Deno maintains outside individual
//! projects.
//!
//! `js.deno_cache` targets Deno's remote-dependency cache at
//! `~/Library/Caches/deno` (macOS) or `~/.cache/deno` (Linux / XDG).
//! It is marked **caution** because Deno has no `node_modules`; remote
//! deps live only in this cache, so offline-mode projects fail until a
//! network-connected `deno cache` runs.
//!
//! This is a global-path rule — the path itself is the marker, not a
//! project marker. `apply_path_safety` whitelists it via
//! `rules::is_global_rule` so the generic runtime-tree block doesn't
//! demote them.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    // Deno: ~/Library/Caches/deno (macOS) or ~/.cache/deno (Linux/XDG).
    if name == "deno" && is_user_cache_parent(project_dir) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "js.deno_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["Deno remote-dependency cache".to_string()],
            warnings: vec![
                "Deno has no node_modules; remote dependencies are \
                 stored only in this global cache. Deleting it makes \
                 offline-mode projects fail until a network-connected \
                 `deno cache` runs."
                    .to_string(),
            ],
            restore_hint: "Run `deno cache --reload`; Deno will refetch on the next run"
                .to_string(),
        });
    }

    None
}

fn is_user_cache_parent(dir: &Path) -> bool {
    parent_ends_with(dir, &["Library", "Caches"]) || parent_ends_with(dir, &[".cache"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_deno_under_macos_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("deno");
        let draft = classify(&parent, "deno", &path).expect("should classify");
        assert_eq!(draft.rule_id, "js.deno_cache");
        assert_eq!(draft.safety, Safety::Caution);
        assert!(
            draft.warnings.iter().any(|w| w.contains("offline")),
            "Deno draft should warn about offline-build risk; got {:?}",
            draft.warnings
        );
        assert!(draft.restore_hint.contains("deno cache"));
    }

    #[test]
    fn classifies_deno_under_xdg_cache() {
        let parent = PathBuf::from("/home/me/.cache");
        let path = parent.join("deno");
        let draft = classify(&parent, "deno", &path).expect("should classify");
        assert_eq!(draft.rule_id, "js.deno_cache");
    }

    #[test]
    fn rejects_deno_outside_canonical_paths() {
        let parent = PathBuf::from("/Users/me/project");
        let path = parent.join("deno");
        assert!(classify(&parent, "deno", &path).is_none());
    }

    #[test]
    fn rejects_other_names_inside_user_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("Spotify");
        assert!(classify(&parent, "Spotify", &path).is_none());
    }
}
