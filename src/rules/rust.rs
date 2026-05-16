use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::has_marker;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name != "target" || !has_marker(project_dir, "Cargo.toml") {
        return None;
    }

    Some(CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: "rust.target".to_string(),
        category: Category::Build,
        safety: Safety::Safe,
        reasons: vec!["Cargo.toml marker found".to_string()],
        warnings: Vec::new(),
        restore_hint: "Run cargo build or cargo test".to_string(),
    })
}
