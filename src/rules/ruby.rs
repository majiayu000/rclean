use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::is_ruby_project;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if !is_ruby_project(project_dir) {
        return None;
    }

    match name {
        ".bundle" => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "ruby.bundle".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["Ruby project marker found".to_string()],
            warnings: vec![".bundle can contain local Bundler configuration".to_string()],
            restore_hint: "Run bundle install".to_string(),
        }),
        "vendor" if path.join("bundle").is_dir() => Some(CandidateDraft {
            path: path.join("bundle"),
            name: "vendor/bundle".to_string(),
            rule_id: "ruby.vendor_bundle".to_string(),
            category: Category::Deps,
            safety: Safety::Caution,
            reasons: vec!["Ruby project marker and vendor/bundle found".to_string()],
            warnings: vec!["vendor/bundle may contain intentionally vendored gems".to_string()],
            restore_hint: "Run bundle install".to_string(),
        }),
        _ => None,
    }
}
