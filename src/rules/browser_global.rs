//! Puppeteer global cache rule.
//!
//! Issue #100 extension to the v0.2 "developer-grade mole" expansion.
//! Adds coverage for a tooling cache that routinely accumulates
//! hundreds of MB outside individual projects.
//!
//! `browser.puppeteer` targets `~/.cache/puppeteer` and
//! `~/Library/Caches/puppeteer`. It is marked **caution** because
//! Puppeteer (2024+) downloads Chrome for Testing here; scripts fail
//! until `npx puppeteer browsers install chrome` rebuilds it.
//!
//! This is a global-path rule. `apply_path_safety` whitelists it via
//! `rules::is_global_rule` so the generic runtime-tree
//! block doesn't demote them.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if !is_user_cache_parent(project_dir) {
        return None;
    }

    match name {
        "puppeteer" => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "browser.puppeteer".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["Puppeteer Chrome for Testing cache".to_string()],
            warnings: vec![
                "Puppeteer 2024+ downloads Chrome for Testing here; \
                 scripts that drive Puppeteer will fail until \
                 `npx puppeteer browsers install chrome` re-downloads it."
                    .to_string(),
            ],
            restore_hint:
                "Run `npx puppeteer browsers install chrome` to re-download Chrome for Testing"
                    .to_string(),
        }),
        _ => None,
    }
}

fn is_user_cache_parent(dir: &Path) -> bool {
    parent_ends_with(dir, &["Library", "Caches"]) || parent_ends_with(dir, &[".cache"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_puppeteer_under_xdg_cache() {
        let parent = PathBuf::from("/home/me/.cache");
        let path = parent.join("puppeteer");
        let draft = classify(&parent, "puppeteer", &path).expect("should classify");
        assert_eq!(draft.rule_id, "browser.puppeteer");
        assert_eq!(draft.safety, Safety::Caution);
        assert!(draft.restore_hint.contains("puppeteer browsers install"));
    }

    #[test]
    fn classifies_puppeteer_under_macos_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("puppeteer");
        let draft = classify(&parent, "puppeteer", &path).expect("should classify");
        assert_eq!(draft.rule_id, "browser.puppeteer");
    }

    #[test]
    fn rejects_puppeteer_outside_canonical_paths() {
        let parent = PathBuf::from("/srv/random");
        let path = parent.join("puppeteer");
        assert!(classify(&parent, "puppeteer", &path).is_none());
    }

    #[test]
    fn rejects_unrelated_names_in_user_cache() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("Spotify");
        assert!(classify(&parent, "Spotify", &path).is_none());
    }
}
