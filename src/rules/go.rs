use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::has_marker;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name != "vendor" || !has_marker(project_dir, "go.mod") {
        return None;
    }

    Some(CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: "go.vendor".to_string(),
        category: Category::Deps,
        safety: Safety::Caution,
        reasons: vec!["go.mod marker found".to_string()],
        warnings: vec!["vendor may contain intentionally checked-in dependencies".to_string()],
        restore_hint: "Run go mod vendor".to_string(),
    })
}
