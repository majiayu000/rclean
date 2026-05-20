//! Xcode global cache rules (macOS).
//!
//! Phase 1 of the v0.2 "developer-grade mole" expansion. Adds rules
//! for caches Xcode produces in the user Library tree that are
//! globally unique paths — i.e. not classified by a project-local
//! marker, but by the path itself.
//!
//! Rules:
//! - `xcode.derived_data` — `~/Library/Developer/Xcode/DerivedData`.
//!   Build artifacts; safe to delete, Xcode rebuilds on next build.
//! - `xcode.simulators` — `~/Library/Developer/CoreSimulator`. iOS
//!   Simulator devices/runtimes/caches. Caution — deleting forces
//!   Xcode to recreate every simulator on next run, which is a
//!   multi-minute operation per device.
//!
//! Both classify cross-platform — the canonical paths only exist
//! on macOS, so non-macOS systems naturally never match.
//!
//! See `docs/specs/v0.2-developer-mole.md` §3.1.
//!
//! Not handled here (deferred to v0.2.1): per-simulator in-use
//! detection (only delete devices not currently booted or paired).
//! SPEC §3.1 calls this out as `xcode.simulators_unused`. Needs
//! `xcrun simctl list` subprocess + per-device path-anchored
//! dispatch. v0.2.0 ships the coarser "whole CoreSimulator/ tree"
//! candidate, which still recovers the bulk of the disk space at
//! the cost of slower next-run simulator recreation.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name == "DerivedData" && parent_ends_with(project_dir, &["Library", "Developer", "Xcode"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "xcode.derived_data".to_string(),
            category: Category::Build,
            safety: Safety::Safe,
            reasons: vec!["Xcode DerivedData cache".to_string()],
            warnings: Vec::new(),
            restore_hint: "Xcode will repopulate it on the next build".to_string(),
        });
    }

    if name == "CoreSimulator" && parent_ends_with(project_dir, &["Library", "Developer"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "xcode.simulators".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["iOS Simulator devices, runtimes, and caches".to_string()],
            warnings: Vec::new(),
            restore_hint:
                "Xcode will recreate simulators on the next iOS app run (multi-minute per device)"
                    .to_string(),
        });
    }

    None
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

    #[test]
    fn classifies_core_simulator_under_library_developer() {
        let parent = PathBuf::from("/Users/me/Library/Developer");
        let path = parent.join("CoreSimulator");
        let draft = classify(&parent, "CoreSimulator", &path).expect("should classify");
        assert_eq!(draft.rule_id, "xcode.simulators");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Caution);
        assert!(draft.restore_hint.contains("simulator"));
    }

    #[test]
    fn rejects_core_simulator_outside_library_developer() {
        let parent = PathBuf::from("/Users/me/project");
        let path = parent.join("CoreSimulator");
        assert!(classify(&parent, "CoreSimulator", &path).is_none());
    }
}
