//! Homebrew global cache rules.
//!
//! `homebrew.downloads` targets Homebrew's downloaded bottle/source
//! archive cache at an exact `Homebrew/downloads` anchor. It avoids
//! broad Homebrew cellar, prefix, or metadata cleanup.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name == "downloads" && is_homebrew_cache_parent(project_dir) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "homebrew.downloads".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["Homebrew bottle/source download cache".to_string()],
            warnings: Vec::new(),
            restore_hint:
                "Homebrew will redownload bottles or source archives on the next install/upgrade"
                    .to_string(),
        });
    }

    None
}

fn is_homebrew_cache_parent(dir: &Path) -> bool {
    parent_ends_with(dir, &["Library", "Caches", "Homebrew"])
        || parent_ends_with(dir, &[".cache", "Homebrew"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_homebrew_downloads_under_macos_cache() {
        let parent = PathBuf::from("/Users/me/Library/Caches/Homebrew");
        let path = parent.join("downloads");
        let draft = classify(&parent, "downloads", &path).expect("should classify");
        assert_eq!(draft.rule_id, "homebrew.downloads");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.restore_hint.contains("redownload"));
    }

    #[test]
    fn classifies_homebrew_downloads_under_xdg_cache() {
        let parent = PathBuf::from("/home/me/.cache/Homebrew");
        let path = parent.join("downloads");
        let draft = classify(&parent, "downloads", &path).expect("should classify");
        assert_eq!(draft.rule_id, "homebrew.downloads");
        assert_eq!(draft.safety, Safety::Safe);
    }

    #[test]
    fn rejects_downloads_outside_homebrew_cache() {
        let parent = PathBuf::from("/Users/me/Downloads/Homebrew");
        let path = parent.join("downloads");
        assert!(classify(&parent, "downloads", &path).is_none());
    }

    #[test]
    fn rejects_other_names_inside_homebrew_cache() {
        let parent = PathBuf::from("/Users/me/Library/Caches/Homebrew");
        let path = parent.join("api");
        assert!(classify(&parent, "api", &path).is_none());
    }
}
