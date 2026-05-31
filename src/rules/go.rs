use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::{has_marker, parent_ends_with};

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name == "download" && parent_ends_with(project_dir, &["pkg", "mod", "cache"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "go.module_download_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["Go module download cache".to_string()],
            warnings: Vec::new(),
            restore_hint: "Go will redownload modules on the next build or test".to_string(),
        });
    }

    if name == "go-build" && is_go_build_cache_parent(project_dir) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "go.build_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["Go build cache".to_string()],
            warnings: Vec::new(),
            restore_hint: "Go will rebuild cached objects on the next build or test".to_string(),
        });
    }

    if name == "vendor" && has_marker(project_dir, "go.mod") {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "go.vendor".to_string(),
            category: Category::Deps,
            safety: Safety::Caution,
            reasons: vec!["go.mod marker found".to_string()],
            warnings: vec!["vendor may contain intentionally checked-in dependencies".to_string()],
            restore_hint: "Run go mod vendor".to_string(),
        });
    }

    None
}

fn is_go_build_cache_parent(path: &Path) -> bool {
    parent_ends_with(path, &["Library", "Caches"])
        || parent_ends_with(path, &[".cache"])
        || parent_ends_with(path, &["AppData", "Local"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_go_module_download_cache() {
        let parent = PathBuf::from("/Users/me/go/pkg/mod/cache");
        let path = parent.join("download");
        let draft = classify(&parent, "download", &path).expect("should classify");

        assert_eq!(draft.rule_id, "go.module_download_cache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
    }

    #[test]
    fn classifies_go_build_cache_under_macos_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("go-build");
        let draft = classify(&parent, "go-build", &path).expect("should classify");

        assert_eq!(draft.rule_id, "go.build_cache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
    }

    #[test]
    fn classifies_go_build_cache_under_xdg_cache() {
        let parent = PathBuf::from("/home/me/.cache");
        let path = parent.join("go-build");
        let draft = classify(&parent, "go-build", &path).expect("should classify");

        assert_eq!(draft.rule_id, "go.build_cache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
    }

    #[test]
    fn classifies_go_build_cache_under_windows_local_app_data() {
        let parent = PathBuf::from("C:/Users/me/AppData/Local");
        let path = parent.join("go-build");
        let draft = classify(&parent, "go-build", &path).expect("should classify");

        assert_eq!(draft.rule_id, "go.build_cache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
    }

    #[test]
    fn rejects_download_outside_go_module_cache() {
        let parent = PathBuf::from("/Users/me/Downloads/cache");
        let path = parent.join("download");

        assert!(classify(&parent, "download", &path).is_none());
    }
}
