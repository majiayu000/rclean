//! Playwright browser-binaries cache rule.
//!
//! Adds `playwright.browsers` for the Chromium / Firefox / WebKit
//! downloads Playwright provisions globally during
//! `npx playwright install`. The cache lives at
//! `~/Library/Caches/ms-playwright/` on macOS and
//! `~/.cache/ms-playwright/` on Linux. Windows uses a different
//! convention (LocalAppData) and is out of scope for v0.3.
//!
//! Like Cargo registry crates or pip wheels, these are
//! redistributable artifacts maintained by a language-ecosystem
//! tool — safe to delete; `playwright install` re-downloads on
//! next run. The Playwright Browsers are not the same thing as a
//! locally-launched Chrome / Safari and have no user profile data.
//!
//! See `docs/specs/v0.3-developer-toolchain-extra.md` §3.1.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name != "ms-playwright" {
        return None;
    }

    #[cfg(target_os = "macos")]
    let anchor_matches = parent_ends_with(project_dir, &["Library", "Caches"]);

    #[cfg(not(target_os = "macos"))]
    let anchor_matches = parent_ends_with(project_dir, &[".cache"]);

    if !anchor_matches {
        return None;
    }

    Some(CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: "playwright.browsers".to_string(),
        category: Category::Cache,
        safety: Safety::Safe,
        reasons: vec!["Playwright downloaded browser binaries".to_string()],
        warnings: Vec::new(),
        restore_hint: "Playwright will redownload browsers on next `npx playwright install`"
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[cfg(target_os = "macos")]
    #[test]
    fn classifies_ms_playwright_under_library_caches_on_macos() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("ms-playwright");
        let draft = classify(&parent, "ms-playwright", &path).expect("should classify on macOS");
        assert_eq!(draft.rule_id, "playwright.browsers");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.restore_hint.contains("playwright"));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn classifies_ms_playwright_under_dot_cache_on_linux() {
        let parent = PathBuf::from("/home/me/.cache");
        let path = parent.join("ms-playwright");
        let draft = classify(&parent, "ms-playwright", &path).expect("should classify on Linux");
        assert_eq!(draft.rule_id, "playwright.browsers");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
    }

    #[test]
    fn rejects_ms_playwright_outside_known_anchor() {
        let parent = PathBuf::from("/Users/me/projects/foo");
        let path = parent.join("ms-playwright");
        assert!(classify(&parent, "ms-playwright", &path).is_none());
    }

    #[test]
    fn rejects_other_names_inside_caches() {
        #[cfg(target_os = "macos")]
        let parent = PathBuf::from("/Users/me/Library/Caches");
        #[cfg(not(target_os = "macos"))]
        let parent = PathBuf::from("/home/me/.cache");

        let path = parent.join("pip");
        assert!(classify(&parent, "pip", &path).is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn rejects_linux_anchor_on_macos() {
        // Defensive: on macOS the rule must NOT match the Linux
        // `~/.cache/ms-playwright` layout, otherwise we'd risk
        // double-counting if a user happens to have both directories.
        let parent = PathBuf::from("/Users/me/.cache");
        let path = parent.join("ms-playwright");
        assert!(classify(&parent, "ms-playwright", &path).is_none());
    }
}
