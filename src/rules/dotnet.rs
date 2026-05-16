use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::is_dotnet_project;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if !is_dotnet_project(project_dir) {
        return None;
    }

    match name {
        "bin" | "obj" => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: format!("dotnet.{name}"),
            category: Category::Build,
            safety: Safety::Safe,
            reasons: vec![".NET project marker found".to_string()],
            warnings: Vec::new(),
            restore_hint: "Run dotnet build".to_string(),
        }),
        _ => None,
    }
}
