//! Android SDK cache rules.
//!
//! These rules deliberately target only narrow SDK Manager / legacy AGP cache
//! anchors. They do not classify SDK components, system images, AVDs, NDKs, or
//! installed build-tools as cleanup candidates.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name == ".downloadIntermediates" && is_android_sdk_root(project_dir) {
        return Some(android_sdk_candidate(
            path,
            name,
            "android_sdk.download_intermediates",
            "Android SDK Manager download intermediates",
            "Close Android Studio or sdkmanager before removing SDK download intermediates",
            "sdkmanager or Android Studio will recreate downloads when needed",
        ));
    }

    if name == "build-cache" && parent_ends_with(project_dir, &[".android"]) {
        return Some(android_sdk_candidate(
            path,
            name,
            "android_sdk.legacy_build_cache",
            "legacy Android Gradle Plugin build cache",
            "Deleting the legacy Android build cache can slow the next Android build",
            "Android Gradle Plugin will rebuild cache entries on the next build",
        ));
    }

    None
}

fn is_android_sdk_root(parent: &Path) -> bool {
    parent_ends_with(parent, &["Library", "Android", "sdk"])
        || parent_ends_with(parent, &["Android", "Sdk"])
        || parent_ends_with(parent, &["Android", "sdk"])
        || parent_ends_with(parent, &["AppData", "Local", "Android", "Sdk"])
}

fn android_sdk_candidate(
    path: &Path,
    name: &str,
    rule_id: &str,
    reason: &str,
    warning: &str,
    restore_hint: &str,
) -> CandidateDraft {
    CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: rule_id.to_string(),
        category: Category::Cache,
        safety: Safety::Caution,
        reasons: vec![reason.to_string()],
        warnings: vec![warning.to_string()],
        restore_hint: restore_hint.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_download_intermediates_under_exact_sdk_roots() {
        for parent in [
            "/Users/me/Library/Android/sdk",
            "/home/me/Android/Sdk",
            "C:/Users/me/AppData/Local/Android/Sdk",
        ] {
            let parent = PathBuf::from(parent);
            let path = parent.join(".downloadIntermediates");
            let draft =
                classify(&parent, ".downloadIntermediates", &path).expect("should classify");
            assert_eq!(draft.rule_id, "android_sdk.download_intermediates");
            assert_eq!(draft.safety, Safety::Caution);
        }
    }

    #[test]
    fn classifies_legacy_build_cache_under_dot_android() {
        let parent = PathBuf::from("/Users/me/.android");
        let path = parent.join("build-cache");
        let draft = classify(&parent, "build-cache", &path).expect("should classify");
        assert_eq!(draft.rule_id, "android_sdk.legacy_build_cache");
        assert_eq!(draft.safety, Safety::Caution);
    }

    #[test]
    fn rejects_sdk_components_and_noncanonical_cache_names() {
        for (parent, name) in [
            ("/Users/me/Library/Android/sdk", "build-tools"),
            ("/Users/me/Library/Android/sdk", "system-images"),
            ("/Users/me/Library/Android/sdk", "caches"),
            ("/Users/me/project", ".downloadIntermediates"),
            ("/Users/me/project", "build-cache"),
        ] {
            let parent = PathBuf::from(parent);
            assert!(classify(&parent, name, &parent.join(name)).is_none());
        }
    }
}
