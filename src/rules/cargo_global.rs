//! Cargo global cache rules (cross-platform).
//!
//! Phase 1 of the v0.2 "developer-grade mole" expansion. Adds two
//! rules for caches Cargo maintains under the user toolchain root
//! (`~/.cargo/`):
//!
//! - `cargo.registry_cache` — `<.cargo>/registry/cache/<source>/`.
//!   Holds downloaded `.crate` archives. Safe to delete; Cargo
//!   redownloads on the next build.
//! - `cargo.git_db` — `<.cargo>/git/db/`. Holds bare clones of
//!   git-dependency sources. Safe to delete; Cargo re-clones on
//!   the next build.
//!
//! These are *global-path* rules (the path itself is the marker,
//! not a project marker like Cargo.toml). `apply_path_safety`
//! whitelists them via `rules::is_global_rule` so the generic
//! runtime/system-path block (which catches `.cargo`) doesn't
//! demote them to Blocked.
//!
//! See `docs/specs/v0.2-developer-mole.md` §3.1.
//!
//! Note: `<.cargo>/registry/src/` is intentionally not handled here.
//! Its candidate name is "src" which is far too common across project
//! trees and would slow the walker. A future PR can pick it up via a
//! path-anchored dispatch path.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name == "cache" && parent_ends_with(project_dir, &[".cargo", "registry"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "cargo.registry_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["Cargo registry crate cache".to_string()],
            warnings: Vec::new(),
            restore_hint: "Cargo will redownload crates on the next build".to_string(),
        });
    }

    if name == "db" && parent_ends_with(project_dir, &[".cargo", "git"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "cargo.git_db".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["Cargo git-dependency clones".to_string()],
            warnings: Vec::new(),
            restore_hint: "Cargo will re-clone git dependencies on the next build".to_string(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_registry_cache() {
        let parent = PathBuf::from("/Users/me/.cargo/registry");
        let path = parent.join("cache");
        let draft = classify(&parent, "cache", &path).expect("should classify");
        assert_eq!(draft.rule_id, "cargo.registry_cache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.restore_hint.contains("Cargo"));
    }

    #[test]
    fn classifies_git_db() {
        let parent = PathBuf::from("/Users/me/.cargo/git");
        let path = parent.join("db");
        let draft = classify(&parent, "db", &path).expect("should classify");
        assert_eq!(draft.rule_id, "cargo.git_db");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Safe);
    }

    #[test]
    fn rejects_cache_outside_cargo_registry() {
        let parent = PathBuf::from("/Users/me/project/some/path");
        let path = parent.join("cache");
        assert!(classify(&parent, "cache", &path).is_none());
    }

    #[test]
    fn rejects_db_outside_cargo_git() {
        let parent = PathBuf::from("/Users/me/.cargo/registry");
        let path = parent.join("db");
        assert!(classify(&parent, "db", &path).is_none());
    }

    #[test]
    fn rejects_other_names_inside_cargo_registry() {
        let parent = PathBuf::from("/Users/me/.cargo/registry");
        let path = parent.join("index");
        assert!(classify(&parent, "index", &path).is_none());
    }

    #[test]
    fn rejects_short_paths() {
        let parent = PathBuf::from("/registry");
        let path = parent.join("cache");
        assert!(classify(&parent, "cache", &path).is_none());
    }
}
