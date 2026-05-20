//! Xcode global cache rules (macOS).
//!
//! Phase 1 of the v0.2 "developer-grade mole" expansion. Adds rules
//! for caches Xcode produces in the user Library tree that are
//! globally unique paths — i.e. not classified by a project-local
//! marker, but by the path itself.
//!
//! Currently only `DerivedData` is implemented. The discriminator is
//! the canonical path suffix `Library/Developer/Xcode/DerivedData`;
//! this matches on macOS (where the directory actually exists) and
//! is a no-op anywhere else because the path never appears.
//!
//! See `docs/specs/v0.2-developer-mole.md` §3.1.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name != "DerivedData" {
        return None;
    }
    if !parent_ends_with(project_dir, &["Library", "Developer", "Xcode"]) {
        return None;
    }

    Some(CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: "xcode.derived_data".to_string(),
        category: Category::Build,
        safety: Safety::Safe,
        reasons: vec!["Xcode DerivedData cache".to_string()],
        warnings: Vec::new(),
        restore_hint: "Xcode will repopulate it on the next build".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_xcode_derived_data_under_library_developer_xcode() {
        let parent = PathBuf::from("/Users/me/Library/Developer/Xcode");
        let path = parent.join("DerivedData");
        let draft = classify(&parent, "DerivedData", &path).expect("should classify");
        assert_eq!(draft.rule_id, "xcode.derived_data");
        assert_eq!(draft.category, Category::Build);
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.restore_hint.contains("Xcode"));
    }

    #[test]
    fn rejects_derived_data_outside_xcode_developer() {
        let parent = PathBuf::from("/tmp/random/Xcode");
        let path = parent.join("DerivedData");
        assert!(classify(&parent, "DerivedData", &path).is_none());
    }

    #[test]
    fn rejects_other_names_inside_xcode_developer() {
        let parent = PathBuf::from("/Users/me/Library/Developer/Xcode");
        let path = parent.join("UserData");
        assert!(classify(&parent, "UserData", &path).is_none());
    }

    #[test]
    fn rejects_short_paths() {
        let parent = PathBuf::from("/Xcode");
        let path = parent.join("DerivedData");
        assert!(classify(&parent, "DerivedData", &path).is_none());
    }
}
