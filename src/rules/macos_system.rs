//! Narrow macOS whole-machine cache rules.
//!
//! These rules intentionally match only exact, empirically observed
//! rebuildable subdirectories. They must never broaden into deleting
//! `/private/var/folders`, `T`, `C`, `X`, `~/Library/Containers`, or
//! application support roots wholesale.

use std::path::Path;

use crate::model::CandidateDraft;
#[cfg(target_os = "macos")]
use crate::model::{Category, Safety};
#[cfg(target_os = "macos")]
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (project_dir, name, path);
        return None;
    }

    #[cfg(target_os = "macos")]
    classify_macos(project_dir, name, path)
}

#[cfg(target_os = "macos")]
fn classify_macos(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name == "com.google.Chrome.code_sign_clone"
        && is_private_var_folders_parent(project_dir, "X")
    {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "macos.chrome_code_sign_clone".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["Chrome/macOS temporary code-sign clone data".to_string()],
            warnings: vec![
                "Skip this candidate while Chrome or related helpers have files open".to_string(),
            ],
            restore_hint: "Chrome/macOS will recreate temporary code-sign clone data when needed"
                .to_string(),
        });
    }

    if name.starts_with("remem-dry-run-") && is_private_var_folders_parent(project_dir, "T") {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "macos.remem_dry_run_tmp".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["Temporary remem dry-run sqlite workspace".to_string()],
            warnings: vec!["Skip this candidate while a process still has it open".to_string()],
            restore_hint:
                "No restore needed; persistent remem state lives outside this dry-run temp path"
                    .to_string(),
        });
    }

    if name == "videos"
        && parent_ends_with(
            project_dir,
            &[
                "Library",
                "Application Support",
                "com.apple.wallpaper",
                "aerials",
            ],
        )
    {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "apple.wallpaper_aerial_videos".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["macOS aerial wallpaper downloaded video cache".to_string()],
            warnings: vec![
                "Wallpaper assets may redownload and wallpaper settings may churn".to_string(),
            ],
            restore_hint: "System Settings > Wallpaper will redownload selected aerials on demand"
                .to_string(),
        });
    }

    if name == "OptGuideOnDeviceModel"
        && parent_ends_with(
            project_dir,
            &["Library", "Application Support", "Google", "Chrome"],
        )
    {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "chrome.opt_guide_model".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["Chrome local optimization model cache".to_string()],
            warnings: vec![
                "Close Chrome first; the model may be redownloaded or rebuilt".to_string(),
            ],
            restore_hint: "Chrome can redownload or rebuild this optimization model when needed"
                .to_string(),
        });
    }

    if name == "update"
        && parent_ends_with(
            project_dir,
            &["Library", "Application Support", "LarkInternational"],
        )
    {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "app.lark_update".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["Lark/Feishu downloaded update payloads".to_string()],
            warnings: vec!["Close Lark/Feishu first; update payloads may redownload".to_string()],
            restore_hint: "Lark/Feishu will download future updates again when needed".to_string(),
        });
    }

    None
}

pub(crate) fn is_dynamic_candidate_name(name: &str) -> bool {
    name.starts_with("remem-dry-run-")
}

#[cfg(target_os = "macos")]
fn is_private_var_folders_parent(parent: &Path, expected_leaf: &str) -> bool {
    let components: Vec<&str> = parent
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect();
    if components.len() < 5 {
        return false;
    }
    let len = components.len();
    if components[len - 1] != expected_leaf || components[len - 4] != "folders" {
        return false;
    }
    (len == 6 && components[0] == "private" && components[1] == "var")
        || (len == 5 && components[0] == "var")
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_chrome_code_sign_clone_only_under_private_var_x() {
        let parent = PathBuf::from("/private/var/folders/zz/abcd1234/X");
        let path = parent.join("com.google.Chrome.code_sign_clone");
        let draft = classify(&parent, "com.google.Chrome.code_sign_clone", &path)
            .expect("should classify code sign clone");
        assert_eq!(draft.rule_id, "macos.chrome_code_sign_clone");
        assert_eq!(draft.safety, Safety::Safe);

        let wrong_parent = PathBuf::from("/private/var/folders/zz/abcd1234/T");
        assert!(
            classify(
                &wrong_parent,
                "com.google.Chrome.code_sign_clone",
                &wrong_parent.join("com.google.Chrome.code_sign_clone")
            )
            .is_none()
        );
    }

    #[test]
    fn classifies_remem_dry_run_tmp_only_under_private_var_t() {
        let parent = PathBuf::from("/private/var/folders/zz/abcd1234/T");
        let path = parent.join("remem-dry-run-123");
        let draft =
            classify(&parent, "remem-dry-run-123", &path).expect("should classify remem tmp");
        assert_eq!(draft.rule_id, "macos.remem_dry_run_tmp");
        assert_eq!(draft.safety, Safety::Safe);

        let wrong_parent = PathBuf::from("/tmp");
        assert!(classify(&wrong_parent, "remem-dry-run-123", &path).is_none());
    }

    #[test]
    fn rejects_private_var_bucket_names() {
        let parent = PathBuf::from("/private/var/folders/zz/abcd1234");
        for name in ["T", "C", "X"] {
            assert!(
                classify(&parent, name, &parent.join(name)).is_none(),
                "must not classify broad /private/var/folders bucket {name}"
            );
        }
    }

    #[test]
    fn classifies_caution_macos_app_candidates() {
        let wallpaper =
            PathBuf::from("/Users/me/Library/Application Support/com.apple.wallpaper/aerials");
        let draft = classify(&wallpaper, "videos", &wallpaper.join("videos"))
            .expect("should classify wallpaper videos");
        assert_eq!(draft.rule_id, "apple.wallpaper_aerial_videos");
        assert_eq!(draft.safety, Safety::Caution);

        let chrome = PathBuf::from("/Users/me/Library/Application Support/Google/Chrome");
        let draft = classify(
            &chrome,
            "OptGuideOnDeviceModel",
            &chrome.join("OptGuideOnDeviceModel"),
        )
        .expect("should classify Chrome model cache");
        assert_eq!(draft.rule_id, "chrome.opt_guide_model");
        assert_eq!(draft.safety, Safety::Caution);
    }
}
