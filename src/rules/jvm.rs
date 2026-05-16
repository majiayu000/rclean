use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::{has_marker, is_gradle_project};

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    match name {
        "target" if has_marker(project_dir, "pom.xml") => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "java.maven_target".to_string(),
            category: Category::Build,
            safety: Safety::Safe,
            reasons: vec!["pom.xml marker found".to_string()],
            warnings: Vec::new(),
            restore_hint: "Run Maven build".to_string(),
        }),
        ".gradle" if is_gradle_project(project_dir) => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "java.gradle_cache_local".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["Gradle marker found".to_string()],
            warnings: vec![".gradle may contain useful local Gradle state".to_string()],
            restore_hint: "Rebuilt by Gradle".to_string(),
        }),
        "build" if is_gradle_project(project_dir) => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "java.gradle_build".to_string(),
            category: Category::Build,
            safety: Safety::Safe,
            reasons: vec!["Gradle marker found".to_string()],
            warnings: Vec::new(),
            restore_hint: "Run Gradle build".to_string(),
        }),
        _ => None,
    }
}
