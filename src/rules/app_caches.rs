//! User-level GUI application cache rules (macOS).
//!
//! v0.3 supersedes the v0.2 §2 Non-Goal "no system-level cache
//! cleanup" for a tightly-scoped set of well-known caches under
//! `~/Library/Caches/*` and `~/Library/Application Support/*`.
//! Each rule has a precise anchor (parent_ends_with) so we never
//! treat `~/Library/Caches/*` as one undifferentiated bucket.
//!
//! Rules:
//! - `app.shipit_caches` — any `~/Library/Caches/<bundle-id>.ShipIt`
//!   directory. Squirrel.Mac (VSCode, Notion, Telegram, ...) leaves
//!   already-applied update packages here.
//! - `chrome.cache` — `~/Library/Caches/Google/Chrome` only. NEVER
//!   matches `~/Library/Application Support/Google/Chrome` (the
//!   user data path that owns bookmarks, passwords, extensions).
//! - `chrome.google_updater` — Chrome's auto-updater historical
//!   state at `~/Library/Application Support/Google/GoogleUpdater`.
//!   Chrome rebuilds it on next launch.
//!
//! See `docs/specs/v0.3-developer-toolchain-extra.md` §3.2 and §6
//! (Chrome cache vs Chrome data anti-collision testing).

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    // ShipIt: any `<bundle-id>.ShipIt` under ~/Library/Caches.
    // The bundle prefix varies (com.microsoft.VSCode, notion.id,
    // com.tdesktop.Telegram, ...), so we match by suffix.
    if name.ends_with(".ShipIt")
        && name.len() > ".ShipIt".len()
        && parent_ends_with(project_dir, &["Library", "Caches"])
    {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "app.shipit_caches".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec![
                "Squirrel.Mac ShipIt installer leftovers (already-applied app updates)".to_string(),
            ],
            warnings: Vec::new(),
            restore_hint:
                "none needed — these are leftover update packages from completed app updates"
                    .to_string(),
        });
    }

    // Chrome HTTP/disk cache. Strict parent anchor — must be under
    // Library/Caches/Google, NEVER Application Support/Google.
    if name == "Chrome" && parent_ends_with(project_dir, &["Library", "Caches", "Google"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "chrome.cache".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["Chrome browser HTTP/disk cache (not user data)".to_string()],
            warnings: Vec::new(),
            restore_hint: "Chrome will repopulate the cache on next browsing".to_string(),
        });
    }

    if name == "LarkInternational" && parent_ends_with(project_dir, &["Library", "Caches"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "app.lark_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["Lark/Feishu rebuildable application cache".to_string()],
            warnings: vec!["Close Lark/Feishu first if it is actively running".to_string()],
            restore_hint: "Lark/Feishu will recreate this cache on next launch".to_string(),
        });
    }

    // Chrome auto-updater state.
    if name == "GoogleUpdater"
        && parent_ends_with(project_dir, &["Library", "Application Support", "Google"])
    {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "chrome.google_updater".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["Chrome auto-updater historical state (will be recreated)".to_string()],
            warnings: Vec::new(),
            restore_hint: "Chrome's updater will recreate it on next launch".to_string(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // --- app.shipit_caches ---

    #[test]
    fn classifies_vscode_shipit() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("com.microsoft.VSCode.ShipIt");
        let draft = classify(&parent, "com.microsoft.VSCode.ShipIt", &path)
            .expect("should classify VSCode ShipIt");
        assert_eq!(draft.rule_id, "app.shipit_caches");
        assert_eq!(draft.safety, Safety::Safe);
    }

    #[test]
    fn classifies_notion_shipit() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("notion.id.ShipIt");
        let draft =
            classify(&parent, "notion.id.ShipIt", &path).expect("should classify Notion ShipIt");
        assert_eq!(draft.rule_id, "app.shipit_caches");
    }

    #[test]
    fn classifies_telegram_shipit() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("com.tdesktop.Telegram.ShipIt");
        assert!(classify(&parent, "com.tdesktop.Telegram.ShipIt", &path).is_some());
    }

    #[test]
    fn rejects_bare_shipit_directory() {
        // Just ".ShipIt" with no prefix — most likely user-created or
        // malformed, not a Squirrel.Mac artifact.
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join(".ShipIt");
        assert!(classify(&parent, ".ShipIt", &path).is_none());
    }

    #[test]
    fn rejects_shipit_outside_library_caches() {
        let parent = PathBuf::from("/Users/me/Desktop");
        let path = parent.join("com.microsoft.VSCode.ShipIt");
        assert!(classify(&parent, "com.microsoft.VSCode.ShipIt", &path).is_none());
    }

    #[test]
    fn rejects_non_shipit_name_in_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("com.microsoft.VSCode");
        assert!(classify(&parent, "com.microsoft.VSCode", &path).is_none());
    }

    // --- chrome.cache ---

    #[test]
    fn classifies_chrome_cache_under_library_caches_google() {
        let parent = PathBuf::from("/Users/me/Library/Caches/Google");
        let path = parent.join("Chrome");
        let draft = classify(&parent, "Chrome", &path).expect("should classify Chrome cache");
        assert_eq!(draft.rule_id, "chrome.cache");
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.reasons[0].contains("cache"));
    }

    /// CRITICAL anti-collision test (v0.3 §6, §9 high-risk row):
    /// the Chrome rule MUST NOT match the Application Support copy,
    /// which holds bookmarks, passwords, extensions, and history.
    #[test]
    fn rejects_chrome_under_application_support_google() {
        let parent = PathBuf::from("/Users/me/Library/Application Support/Google");
        let path = parent.join("Chrome");
        assert!(
            classify(&parent, "Chrome", &path).is_none(),
            "MUST NOT match Application Support/Google/Chrome (user data path)"
        );
    }

    #[test]
    fn rejects_chrome_with_wrong_grandparent() {
        // Anchor must be Library/Caches/Google — not Caches alone.
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("Chrome");
        assert!(classify(&parent, "Chrome", &path).is_none());
    }

    #[test]
    fn rejects_chromium_under_library_caches_google() {
        // Defensive: only the literal name "Chrome" matches, not
        // Chromium, Chrome Canary, etc. (those have different cache
        // layouts and may not be safe to wipe blindly).
        let parent = PathBuf::from("/Users/me/Library/Caches/Google");
        let path = parent.join("Chromium");
        assert!(classify(&parent, "Chromium", &path).is_none());
    }

    #[test]
    fn classifies_lark_cache_under_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("LarkInternational");
        let draft =
            classify(&parent, "LarkInternational", &path).expect("should classify Lark cache");
        assert_eq!(draft.rule_id, "app.lark_cache");
        assert_eq!(draft.safety, Safety::Safe);
    }

    // --- chrome.google_updater ---

    #[test]
    fn classifies_google_updater_under_application_support_google() {
        let parent = PathBuf::from("/Users/me/Library/Application Support/Google");
        let path = parent.join("GoogleUpdater");
        let draft =
            classify(&parent, "GoogleUpdater", &path).expect("should classify GoogleUpdater");
        assert_eq!(draft.rule_id, "chrome.google_updater");
        assert_eq!(draft.safety, Safety::Safe);
    }

    #[test]
    fn rejects_google_updater_outside_application_support_google() {
        let parent = PathBuf::from("/Users/me/Library/Caches/Google");
        let path = parent.join("GoogleUpdater");
        assert!(classify(&parent, "GoogleUpdater", &path).is_none());
    }

    #[test]
    fn rejects_short_paths() {
        // Both Chrome and GoogleUpdater require multi-component
        // anchors; very short paths should never match.
        let p = PathBuf::from("/Google");
        assert!(classify(&p, "Chrome", &p.join("Chrome")).is_none());
        assert!(classify(&p, "GoogleUpdater", &p.join("GoogleUpdater")).is_none());
    }
}
