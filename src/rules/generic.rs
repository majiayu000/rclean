use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::has_any_project_marker;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name != "coverage" || !has_any_project_marker(project_dir) {
        return None;
    }

    Some(CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: "generic.coverage".to_string(),
        category: Category::Test,
        safety: Safety::Safe,
        reasons: vec!["project marker found".to_string()],
        warnings: Vec::new(),
        restore_hint: "Re-run the test suite".to_string(),
    })
}
