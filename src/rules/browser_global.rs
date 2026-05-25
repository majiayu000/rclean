//! Browser automation and pre-commit global cache rules.
//!
//! Issue #100 extension to the v0.2 "developer-grade mole" expansion.
//! Adds three rules for tooling caches that routinely accumulate
//! hundreds of MB to multiple GB outside individual projects.
//!
//! - `browser.playwright` — `~/Library/Caches/ms-playwright` (macOS)
//!   or `~/.cache/ms-playwright` (Linux/XDG). Marked **caution**:
//!   downloads Chromium/Firefox/WebKit per project; tests fail until
//!   `npx playwright install` rebuilds it.
//! - `browser.puppeteer` — `~/.cache/puppeteer`. Marked **caution**:
//!   Puppeteer (2024+) downloads Chrome for Testing here. Tests fail
//!   until `npx puppeteer browsers install chrome` rebuilds it.
//! - `lint.pre_commit` — `~/.cache/pre-commit`. **Safe**: hook
//!   environments rebuild automatically on the next `git commit`.
//!
//! All three are global-path rules. `apply_path_safety` whitelists
//! them via `rules::is_global_rule` so the generic runtime-tree
//! block doesn't demote them.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if !is_user_cache_parent(project_dir) {
        return None;
    }

    match name {
        "ms-playwright" => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "browser.playwright".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["Playwright browser binaries".to_string()],
            warnings: vec![
                "Deleting removes Chromium/Firefox/WebKit binaries; \
                 Playwright tests will fail until `npx playwright install` \
                 re-downloads them (typically 300 MB - 1.5 GB)."
                    .to_string(),
            ],
            restore_hint: "Run `npx playwright install` to re-download browser binaries"
                .to_string(),
        }),
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
        "pre-commit" => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "lint.pre_commit".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["pre-commit per-hook environments".to_string()],
            warnings: Vec::new(),
            restore_hint: "pre-commit will rebuild hook environments on the next `git commit`"
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

    // ---- Playwright ----

    #[test]
    fn classifies_playwright_under_macos_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("ms-playwright");
        let draft = classify(&parent, "ms-playwright", &path).expect("should classify");
        assert_eq!(draft.rule_id, "browser.playwright");
        assert_eq!(draft.safety, Safety::Caution);
        assert!(draft.restore_hint.contains("playwright install"));
    }

    #[test]
    fn classifies_playwright_under_xdg_cache() {
        let parent = PathBuf::from("/home/me/.cache");
        let path = parent.join("ms-playwright");
        let draft = classify(&parent, "ms-playwright", &path).expect("should classify");
        assert_eq!(draft.rule_id, "browser.playwright");
    }

    #[test]
    fn rejects_playwright_outside_canonical_paths() {
        let parent = PathBuf::from("/Users/me/project");
        let path = parent.join("ms-playwright");
        assert!(classify(&parent, "ms-playwright", &path).is_none());
    }

    // ---- Puppeteer ----

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

    // ---- pre-commit ----

    #[test]
    fn classifies_pre_commit_under_xdg_cache() {
        let parent = PathBuf::from("/home/me/.cache");
        let path = parent.join("pre-commit");
        let draft = classify(&parent, "pre-commit", &path).expect("should classify");
        assert_eq!(draft.rule_id, "lint.pre_commit");
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.warnings.is_empty(), "pre-commit is safe, no warnings");
        assert!(draft.restore_hint.contains("git commit"));
    }

    #[test]
    fn classifies_pre_commit_under_macos_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("pre-commit");
        let draft = classify(&parent, "pre-commit", &path).expect("should classify");
        assert_eq!(draft.rule_id, "lint.pre_commit");
    }

    #[test]
    fn rejects_pre_commit_outside_canonical_paths() {
        let parent = PathBuf::from("/Users/me/scratch");
        let path = parent.join("pre-commit");
        assert!(classify(&parent, "pre-commit", &path).is_none());
    }

    // ---- negative shared ----

    #[test]
    fn rejects_unrelated_names_in_user_cache() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("Spotify");
        assert!(classify(&parent, "Spotify", &path).is_none());
    }
}
