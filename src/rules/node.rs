use std::path::{Path, PathBuf};

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::{has_marker, is_node_project};

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    match name {
        "node_modules" if has_marker(project_dir, "package.json") => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "node.node_modules".to_string(),
            category: Category::Deps,
            safety: Safety::Safe,
            reasons: vec!["package.json marker found".to_string()],
            warnings: Vec::new(),
            restore_hint: "Run npm install, pnpm install, yarn install, or bun install".to_string(),
        }),
        ".next" if is_node_project(project_dir) => Some(node_cache_safe(
            path,
            name,
            "node.next",
            Category::Build,
            "Run the Next.js build or dev command",
        )),
        ".turbo" if is_node_project(project_dir) => Some(node_cache_safe(
            path,
            name,
            "node.turbo",
            Category::Cache,
            "Rebuilt by Turborepo",
        )),
        ".vite" if is_node_project(project_dir) => Some(node_cache_safe(
            path,
            name,
            "node.vite",
            Category::Cache,
            "Rebuilt by Vite",
        )),
        ".parcel-cache" if is_node_project(project_dir) => Some(node_cache_safe(
            path,
            name,
            "node.parcel",
            Category::Cache,
            "Rebuilt by Parcel",
        )),
        "build" | "dist" | "out" if is_node_project(project_dir) => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: format!("node.{name}"),
            category: Category::Build,
            safety: Safety::Caution,
            reasons: vec!["Node project marker found".to_string()],
            warnings: vec![format!("{name} is generic and may contain user outputs")],
            restore_hint: "Re-run the project build".to_string(),
        }),
        _ => None,
    }
}

fn node_cache_safe(
    path: &Path,
    name: &str,
    rule_id: &str,
    category: Category,
    restore_hint: &str,
) -> CandidateDraft {
    CandidateDraft {
        path: PathBuf::from(path),
        name: name.to_string(),
        rule_id: rule_id.to_string(),
        category,
        safety: Safety::Safe,
        reasons: vec!["Node project marker found".to_string()],
        warnings: Vec::new(),
        restore_hint: restore_hint.to_string(),
    }
}
