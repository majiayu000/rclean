//! Bun and Deno global cache rules (cross-platform).
//!
//! Issue #103 extension to the v0.2 "developer-grade mole" expansion.
//! Adds two rules for caches that the next-generation JS runtimes
//! maintain outside individual projects.
//!
//! - `js.bun_install_cache` — Bun's package install cache. **Must**
//!   target `~/.bun/install/cache` specifically, not `~/.bun`. The
//!   parent directory `~/.bun` also contains Bun's runtime binary;
//!   cleaning the whole tree would brick the user's Bun install.
//!   Marked **caution**: Bun uses hardlinks on Linux for its install
//!   cache. The safe restore path is `bun pm cache rm`.
//! - `js.deno_cache` — Deno's remote-dependency cache.
//!   `~/Library/Caches/deno` (macOS) or `~/.cache/deno` (Linux / XDG).
//!   Marked **caution**: Deno has no `node_modules`; remote deps live
//!   only in this cache, so offline-mode projects fail until a
//!   network-connected `deno cache` runs.
//!
//! Both are global-path rules — the path itself is the marker, not a
//! project marker. `apply_path_safety` whitelists them via
//! `rules::is_global_rule` so the generic runtime-tree block doesn't
//! demote them.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    // Bun: anchor on the install/cache sub-path, never on ~/.bun itself.
    if name == "cache" && parent_ends_with(project_dir, &[".bun", "install"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "js.bun_install_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["Bun package install cache".to_string()],
            warnings: vec![
                "Bun uses hardlinks on Linux for the install cache; \
                 deleting via `rm -rf` may leave dangling links in \
                 existing projects' `node_modules`. Prefer \
                 `bun pm cache rm` for a safe rebuild."
                    .to_string(),
            ],
            restore_hint: "Bun will repopulate the cache on the next install".to_string(),
        });
    }

    // Deno: ~/Library/Caches/deno (macOS) or ~/.cache/deno (Linux/XDG).
    if name == "deno" && is_user_cache_parent(project_dir) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "js.deno_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["Deno remote-dependency cache".to_string()],
            warnings: vec![
                "Deno has no node_modules; remote dependencies are \
                 stored only in this global cache. Deleting it makes \
                 offline-mode projects fail until a network-connected \
                 `deno cache` runs."
                    .to_string(),
            ],
            restore_hint: "Run `deno cache --reload`; Deno will refetch on the next run"
                .to_string(),
        });
    }

    None
}

fn is_user_cache_parent(dir: &Path) -> bool {
    parent_ends_with(dir, &["Library", "Caches"]) || parent_ends_with(dir, &[".cache"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ---- Bun ----

    #[test]
    fn classifies_bun_install_cache() {
        let parent = PathBuf::from("/Users/me/.bun/install");
        let path = parent.join("cache");
        let draft = classify(&parent, "cache", &path).expect("should classify");
        assert_eq!(draft.rule_id, "js.bun_install_cache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Caution);
        assert!(
            draft.warnings.iter().any(|w| w.contains("hardlinks")),
            "Bun draft should warn about hardlinks; got {:?}",
            draft.warnings
        );
        assert!(draft.restore_hint.contains("Bun"));
    }

    /// The most important safety invariant for this PR: the rule must
    /// **not** match `~/.bun` itself. Cleaning the parent would
    /// destroy the Bun runtime binary.
    #[test]
    fn rejects_bun_runtime_root() {
        let parent = PathBuf::from("/Users/me");
        let path = parent.join(".bun");
        assert!(classify(&parent, ".bun", &path).is_none());
    }

    /// Equally important: a `cache` directory directly under
    /// `~/.bun` (without `install/` in between) is not the install
    /// cache and must not match.
    #[test]
    fn rejects_bun_cache_directly_under_dot_bun() {
        let parent = PathBuf::from("/Users/me/.bun");
        let path = parent.join("cache");
        assert!(classify(&parent, "cache", &path).is_none());
    }

    #[test]
    fn rejects_bun_install_cache_outside_dot_bun() {
        let parent = PathBuf::from("/Users/me/project/install");
        let path = parent.join("cache");
        assert!(classify(&parent, "cache", &path).is_none());
    }

    #[test]
    fn rejects_other_names_under_bun_install() {
        let parent = PathBuf::from("/Users/me/.bun/install");
        let path = parent.join("global");
        assert!(classify(&parent, "global", &path).is_none());
    }

    // ---- Deno ----

    #[test]
    fn classifies_deno_under_macos_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("deno");
        let draft = classify(&parent, "deno", &path).expect("should classify");
        assert_eq!(draft.rule_id, "js.deno_cache");
        assert_eq!(draft.safety, Safety::Caution);
        assert!(
            draft.warnings.iter().any(|w| w.contains("offline")),
            "Deno draft should warn about offline-build risk; got {:?}",
            draft.warnings
        );
        assert!(draft.restore_hint.contains("deno cache"));
    }

    #[test]
    fn classifies_deno_under_xdg_cache() {
        let parent = PathBuf::from("/home/me/.cache");
        let path = parent.join("deno");
        let draft = classify(&parent, "deno", &path).expect("should classify");
        assert_eq!(draft.rule_id, "js.deno_cache");
    }

    #[test]
    fn rejects_deno_outside_canonical_paths() {
        let parent = PathBuf::from("/Users/me/project");
        let path = parent.join("deno");
        assert!(classify(&parent, "deno", &path).is_none());
    }

    #[test]
    fn rejects_other_names_inside_user_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("Spotify");
        assert!(classify(&parent, "Spotify", &path).is_none());
    }
}
