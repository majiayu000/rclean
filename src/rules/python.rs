use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::{is_python_project, is_virtualenv};

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if !is_python_project(project_dir) {
        return None;
    }

    match name {
        ".venv" if is_virtualenv(path) => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "python.venv_dot".to_string(),
            category: Category::Deps,
            safety: Safety::Safe,
            reasons: vec!["Python marker and virtualenv marker found".to_string()],
            warnings: Vec::new(),
            restore_hint: "Recreate the Python environment".to_string(),
        }),
        "venv" => {
            if is_virtualenv(path) {
                Some(CandidateDraft {
                    path: path.to_path_buf(),
                    name: name.to_string(),
                    rule_id: "python.venv_plain".to_string(),
                    category: Category::Deps,
                    safety: Safety::Safe,
                    reasons: vec!["Python marker and virtualenv marker found".to_string()],
                    warnings: Vec::new(),
                    restore_hint: "Recreate the Python environment".to_string(),
                })
            } else {
                Some(CandidateDraft {
                    path: path.to_path_buf(),
                    name: name.to_string(),
                    rule_id: "python.venv_plain".to_string(),
                    category: Category::Deps,
                    safety: Safety::Blocked,
                    reasons: Vec::new(),
                    warnings: vec!["plain venv directory has no virtualenv marker".to_string()],
                    restore_hint: "Not deleted because this is not validated as a virtualenv"
                        .to_string(),
                })
            }
        }
        "__pycache__" => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "python.pycache".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["Python project marker found".to_string()],
            warnings: Vec::new(),
            restore_hint: "Recreated by Python".to_string(),
        }),
        ".pytest_cache" | ".mypy_cache" | ".ruff_cache" | ".tox" => {
            let (rule_id, hint, safety, warnings) = match name {
                ".pytest_cache" => ("python.pytest", "Recreated by pytest", Safety::Safe, vec![]),
                ".mypy_cache" => ("python.mypy", "Recreated by mypy", Safety::Safe, vec![]),
                ".ruff_cache" => ("python.ruff", "Recreated by ruff", Safety::Safe, vec![]),
                _ => (
                    "python.tox",
                    "Recreated by tox",
                    Safety::Caution,
                    vec![".tox may contain expensive local test environments".to_string()],
                ),
            };
            Some(CandidateDraft {
                path: path.to_path_buf(),
                name: name.to_string(),
                rule_id: rule_id.to_string(),
                category: Category::Cache,
                safety,
                reasons: vec!["Python project marker found".to_string()],
                warnings,
                restore_hint: hint.to_string(),
            })
        }
        _ => None,
    }
}
