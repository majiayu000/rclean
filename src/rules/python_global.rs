//! Python global cache rules (cross-platform).
//!
//! Issue #101 extension to the v0.2 "developer-grade mole" expansion.
//! Adds three rules for caches modern Python tooling maintains under
//! the user toolchain root.
//!
//! - `python.uv_cache` — uv's package cache. Two canonical layouts on
//!   macOS: native `~/Library/Caches/uv` AND XDG fallback `~/.cache/uv`.
//!   Both are checked because real users (and the empirical dev box
//!   this rule was authored against) resolve to `~/.cache/uv` when
//!   `XDG_CACHE_HOME` is set or by uv config. Marked **caution**:
//!   uv uses a content-addressable cache with hardlinks/reflinks into
//!   project `.venv` directories, so direct `rm -rf` can leave dangling
//!   links in active venvs. The safe restore path is `uv cache clean`.
//! - `python.poetry_cache` — Poetry's isolated wheels cache.
//!   `~/Library/Caches/pypoetry` (macOS) or `~/.cache/pypoetry`
//!   (Linux / XDG). Safe to delete; Poetry rebuilds on the next
//!   `poetry install`.
//! - `python.pipx_cache` — pipx's ephemeral-run venv cache.
//!   `~/Library/Caches/pipx` (macOS native per platformdirs) or
//!   `~/.cache/pipx` (Linux / XDG). Safe to delete; pipx repopulates
//!   automatically on the next `pipx run`.
//!
//! All three are global-path rules — the path itself is the marker,
//! not a project marker like `pyproject.toml`. `apply_path_safety`
//! whitelists them via `rules::is_global_rule` so the generic
//! runtime-tree block doesn't demote them.
//!
//! See `docs/specs/python-global-caches.md`.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if !is_user_cache_parent(project_dir) {
        return None;
    }

    match name {
        "uv" => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "python.uv_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["uv package cache".to_string()],
            warnings: vec![
                "uv uses hardlinks/reflinks into project .venv directories; \
                 deleting this cache may leave dangling links in active venvs. \
                 Prefer `uv cache clean` for a safe rebuild."
                    .to_string(),
            ],
            restore_hint: "Run `uv cache clean`; uv will repopulate on the next sync".to_string(),
        }),
        "pypoetry" => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "python.poetry_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["Poetry wheels and HTTP cache".to_string()],
            warnings: Vec::new(),
            restore_hint: "Poetry will repopulate the cache on the next install".to_string(),
        }),
        "pipx" => Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "python.pipx_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["pipx ephemeral-run venv cache".to_string()],
            warnings: Vec::new(),
            restore_hint: "pipx will repopulate the cache on the next `pipx run`".to_string(),
        }),
        _ => None,
    }
}

/// Returns true if `dir` matches either canonical user-cache parent:
/// macOS native `~/Library/Caches`, or Linux/XDG `~/.cache`. The same
/// XDG override is widely active on macOS (uv's empirical 2.5 GB cache
/// lives under `~/.cache/uv` even when `~/Library/Caches/uv` is the
/// platform default).
fn is_user_cache_parent(dir: &Path) -> bool {
    parent_ends_with(dir, &["Library", "Caches"]) || parent_ends_with(dir, &[".cache"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_uv_under_macos_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("uv");
        let draft = classify(&parent, "uv", &path).expect("should classify");
        assert_eq!(draft.rule_id, "python.uv_cache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Caution);
        assert!(
            draft.warnings.iter().any(|w| w.contains("hardlinks")),
            "uv draft should warn about hardlinks; got {:?}",
            draft.warnings
        );
        assert!(draft.restore_hint.contains("uv cache clean"));
    }

    #[test]
    fn classifies_uv_under_xdg_cache_on_macos_override() {
        // The issue's empirical evidence: uv resolves to ~/.cache/uv
        // on macOS when XDG_CACHE_HOME is set or by uv config. Both
        // anchors must be covered.
        let parent = PathBuf::from("/Users/me/.cache");
        let path = parent.join("uv");
        let draft = classify(&parent, "uv", &path).expect("should classify");
        assert_eq!(draft.rule_id, "python.uv_cache");
        assert_eq!(draft.safety, Safety::Caution);
    }

    #[test]
    fn classifies_uv_under_xdg_cache_on_linux() {
        let parent = PathBuf::from("/home/me/.cache");
        let path = parent.join("uv");
        let draft = classify(&parent, "uv", &path).expect("should classify");
        assert_eq!(draft.rule_id, "python.uv_cache");
    }

    #[test]
    fn classifies_pypoetry_under_macos_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("pypoetry");
        let draft = classify(&parent, "pypoetry", &path).expect("should classify");
        assert_eq!(draft.rule_id, "python.poetry_cache");
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.restore_hint.contains("Poetry"));
    }

    #[test]
    fn classifies_pypoetry_under_xdg_cache() {
        let parent = PathBuf::from("/home/me/.cache");
        let path = parent.join("pypoetry");
        let draft = classify(&parent, "pypoetry", &path).expect("should classify");
        assert_eq!(draft.rule_id, "python.poetry_cache");
    }

    #[test]
    fn classifies_pipx_under_macos_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("pipx");
        let draft = classify(&parent, "pipx", &path).expect("should classify");
        assert_eq!(draft.rule_id, "python.pipx_cache");
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.restore_hint.contains("pipx"));
    }

    #[test]
    fn classifies_pipx_under_xdg_cache() {
        let parent = PathBuf::from("/home/me/.cache");
        let path = parent.join("pipx");
        let draft = classify(&parent, "pipx", &path).expect("should classify");
        assert_eq!(draft.rule_id, "python.pipx_cache");
    }

    #[test]
    fn rejects_uv_outside_canonical_paths() {
        let parent = PathBuf::from("/Users/me/project");
        let path = parent.join("uv");
        assert!(classify(&parent, "uv", &path).is_none());
    }

    #[test]
    fn rejects_pypoetry_outside_canonical_paths() {
        let parent = PathBuf::from("/Users/me/work");
        let path = parent.join("pypoetry");
        assert!(classify(&parent, "pypoetry", &path).is_none());
    }

    #[test]
    fn rejects_pipx_outside_canonical_paths() {
        let parent = PathBuf::from("/srv/something");
        let path = parent.join("pipx");
        assert!(classify(&parent, "pipx", &path).is_none());
    }

    #[test]
    fn rejects_unrelated_names_inside_library_caches() {
        let parent = PathBuf::from("/Users/me/Library/Caches");
        let path = parent.join("Spotify");
        assert!(classify(&parent, "Spotify", &path).is_none());
    }

    #[test]
    fn rejects_unrelated_names_inside_xdg_cache() {
        let parent = PathBuf::from("/home/me/.cache");
        let path = parent.join("mozilla");
        assert!(classify(&parent, "mozilla", &path).is_none());
    }
}
