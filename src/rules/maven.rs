//! Maven global cache rule (cross-platform).
//!
//! Phase 1 of the v0.2 "developer-grade mole" expansion. Adds one
//! rule for Maven's user-level dependency cache:
//!
//! - `maven.local_repo` — `<.m2>/repository/`. Holds every JAR
//!   Maven has ever downloaded for any project on this host.
//!
//! Safety: caution. Deleting forces a full redownload of every
//! Maven dependency on the next build, which can be very large.
//! Pairs with the existing project-level `java.maven_target`
//! rule (per-project `target/` under `pom.xml`, Safety::Safe);
//! the two don't overlap because this rule's discriminator is
//! `parent_ends_with .m2`.
//!
//! This is a global-path rule. `apply_path_safety` whitelists
//! it via `rules::is_global_rule`.
//!
//! See `docs/specs/v0.2-developer-mole.md` §3.1.
//!
//! SPEC mentions `<.m2>/repository/<old>` filtered by access time
//! older than 90 days. In practice, atime is disabled by default
//! on macOS (noatime) and Linux (relatime), so the access signal
//! is unreliable. v0.2.0 ships the coarse "whole repository/ is
//! one candidate" behavior. A later PR can add mtime-based or
//! per-version stale detection via path-anchored dispatch.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name != "repository" {
        return None;
    }
    if !parent_ends_with(project_dir, &[".m2"]) {
        return None;
    }

    Some(CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: "maven.local_repo".to_string(),
        category: Category::Cache,
        safety: Safety::Caution,
        reasons: vec!["Maven local repository (downloaded JAR cache)".to_string()],
        warnings: Vec::new(),
        restore_hint: "Maven will redownload dependencies on the next build".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_repository_under_dot_m2() {
        let parent = PathBuf::from("/Users/me/.m2");
        let path = parent.join("repository");
        let draft = classify(&parent, "repository", &path).expect("should classify");
        assert_eq!(draft.rule_id, "maven.local_repo");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Caution);
        assert!(draft.restore_hint.contains("Maven"));
    }

    #[test]
    fn rejects_repository_outside_dot_m2() {
        let parent = PathBuf::from("/Users/me/project");
        let path = parent.join("repository");
        assert!(classify(&parent, "repository", &path).is_none());
    }

    #[test]
    fn rejects_other_names_inside_dot_m2() {
        let parent = PathBuf::from("/Users/me/.m2");
        let path = parent.join("settings.xml");
        assert!(classify(&parent, "settings.xml", &path).is_none());
    }
}
