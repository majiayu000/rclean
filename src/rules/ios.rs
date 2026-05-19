use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::has_marker;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name != "Pods" || !has_marker(project_dir, "Podfile") {
        return None;
    }

    Some(CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: "ios.pods".to_string(),
        category: Category::Deps,
        safety: Safety::Safe,
        reasons: vec!["Podfile marker found".to_string()],
        warnings: Vec::new(),
        restore_hint: "Run pod install".to_string(),
    })
}
