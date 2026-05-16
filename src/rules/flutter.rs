use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::has_marker;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if !has_marker(project_dir, "pubspec.yaml") {
        return None;
    }

    match name {
        ".dart_tool" => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "dart.tool".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["pubspec.yaml marker found".to_string()],
            warnings: Vec::new(),
            restore_hint: "Run flutter pub get or dart pub get".to_string(),
        }),
        "build" => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "dart.build".to_string(),
            category: Category::Build,
            safety: Safety::Safe,
            reasons: vec!["pubspec.yaml marker found".to_string()],
            warnings: Vec::new(),
            restore_hint: "Run flutter build or dart build".to_string(),
        }),
        _ => None,
    }
}
