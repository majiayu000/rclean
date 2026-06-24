//! Dart / Flutter global pub-cache rules.
//!
//! These rules deliberately target only exact `~/.pub-cache` anchors:
//! hosted package cache and git dependency cache. They do not scan
//! project source, Flutter SDK state, or arbitrary package directories.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if !parent_ends_with(project_dir, &[".pub-cache"]) {
        return None;
    }

    match name {
        "hosted" => Some(pub_cache_candidate(
            path,
            name,
            "dart.pub_hosted_cache",
            "Dart pub hosted package cache",
            "Hosted packages will redownload on the next `dart pub get` or `flutter pub get`",
        )),
        "git" => Some(pub_cache_candidate(
            path,
            name,
            "dart.pub_git_cache",
            "Dart pub git dependency cache",
            "Git dependencies will reclone on the next `dart pub get` or `flutter pub get`",
        )),
        _ => None,
    }
}

fn pub_cache_candidate(
    path: &Path,
    name: &str,
    rule_id: &str,
    reason: &str,
    restore_hint: &str,
) -> CandidateDraft {
    CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: rule_id.to_string(),
        category: Category::Cache,
        safety: Safety::Caution,
        reasons: vec![reason.to_string()],
        warnings: vec![
            "Dart pub-cache entries are global dependency caches; deleting them forces redownload/reclone and can break offline builds"
                .to_string(),
        ],
        restore_hint: restore_hint.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_pub_hosted_cache() {
        let parent = PathBuf::from("/Users/me/.pub-cache");
        let path = parent.join("hosted");
        let draft = classify(&parent, "hosted", &path).expect("should classify");
        assert_eq!(draft.rule_id, "dart.pub_hosted_cache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Caution);
        assert!(draft.restore_hint.contains("pub get"));
    }

    #[test]
    fn classifies_pub_git_cache() {
        let parent = PathBuf::from("/Users/me/.pub-cache");
        let path = parent.join("git");
        let draft = classify(&parent, "git", &path).expect("should classify");
        assert_eq!(draft.rule_id, "dart.pub_git_cache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Caution);
        assert!(draft.restore_hint.contains("reclone"));
    }

    #[test]
    fn rejects_hosted_outside_pub_cache() {
        let parent = PathBuf::from("/Users/me/project");
        let path = parent.join("hosted");
        assert!(classify(&parent, "hosted", &path).is_none());
    }

    #[test]
    fn rejects_git_outside_pub_cache() {
        let parent = PathBuf::from("/Users/me/.cache");
        let path = parent.join("git");
        assert!(classify(&parent, "git", &path).is_none());
    }
}
