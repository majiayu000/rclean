//! Gradle global cache rule (cross-platform).
//!
//! Phase 1 of the v0.2 "developer-grade mole" expansion. Adds one
//! rule for Gradle's user-level dependency/build cache:
//!
//! - `gradle.caches` — `<.gradle>/caches/`. Holds downloaded
//!   dependencies, Wrapper installations, build scripts, and
//!   per-version index data for every Gradle version ever used
//!   on the host.
//!
//! Safety: caution. Deleting forces a full redownload of every
//! Gradle dependency on the next build, which is materially
//! larger than e.g. `cargo.registry_cache`. Pairs with the
//! existing project-level `java.gradle_cache_local` rule, which
//! targets a project's per-project `.gradle/` (Safety::Safe).
//!
//! This is a global-path rule. `apply_path_safety` whitelists
//! it via `rules::is_global_rule`.
//!
//! See `docs/specs/v0.2-developer-mole.md` §3.1.
//!
//! Not handled here:
//! - Per-Gradle-version subdir splitting (e.g. only stale
//!   `caches/8.4` while `caches/8.5` is active). SPEC mentions
//!   it; needs the path-anchored dispatch a later PR adds.
//!   v0.2.0 ships the coarse "whole `caches/` is one candidate"
//!   behavior.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name != "caches" {
        return None;
    }
    if !parent_ends_with(project_dir, &[".gradle"]) {
        return None;
    }

    Some(CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: "gradle.caches".to_string(),
        category: Category::Cache,
        safety: Safety::Caution,
        reasons: vec!["Gradle global dependency cache".to_string()],
        warnings: Vec::new(),
        restore_hint: "Gradle will redownload dependencies on the next build".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_gradle_caches_under_dot_gradle() {
        let parent = PathBuf::from("/Users/me/.gradle");
        let path = parent.join("caches");
        let draft = classify(&parent, "caches", &path).expect("should classify");
        assert_eq!(draft.rule_id, "gradle.caches");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Caution);
        assert!(draft.restore_hint.contains("Gradle"));
    }

    #[test]
    fn rejects_caches_outside_dot_gradle() {
        let parent = PathBuf::from("/Users/me/project");
        let path = parent.join("caches");
        assert!(classify(&parent, "caches", &path).is_none());
    }

    #[test]
    fn rejects_other_names_inside_dot_gradle() {
        let parent = PathBuf::from("/Users/me/.gradle");
        let path = parent.join("wrapper");
        assert!(classify(&parent, "wrapper", &path).is_none());
    }
}
