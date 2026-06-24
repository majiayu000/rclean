use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};

pub const RULE_ID: &str = "agent.tmp_worktree";

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if !matches_tmp_worktree_name(name) || !has_tmp_worktree_marker(path) {
        return None;
    }

    Some(CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: RULE_ID.to_string(),
        category: Category::Cache,
        safety: Safety::Caution,
        reasons: vec![
            "temporary agent worktree name and project marker found".to_string(),
            format!("immediate child of tmp root {}", project_dir.display()),
        ],
        warnings: vec![
            "whole worktree cleanup may remove local edits; requires --include-caution".to_string(),
        ],
        restore_hint: "Recreate or reclone the temporary worktree".to_string(),
    })
}

pub fn matches_tmp_worktree_name(name: &str) -> bool {
    name.starts_with("remem-")
        || name.starts_with("rclean-")
        || name.starts_with("loom-")
        || name.contains("review-target")
}

fn has_tmp_worktree_marker(path: &Path) -> bool {
    path.join(".git").symlink_metadata().is_ok()
        || ["Cargo.toml", "package.json", "go.mod", "pyproject.toml"]
            .iter()
            .any(|marker| path.join(marker).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tmp_worktree_name_patterns_are_exact() {
        for name in [
            "remem-alpha",
            "rclean-issue-166",
            "loom-review",
            "codex-review-target-123",
            "review-target",
        ] {
            assert!(matches_tmp_worktree_name(name), "expected {name} to match");
        }

        for name in [
            "remem",
            "rememalpha",
            "xremem-alpha",
            "rclean",
            "rclean_alpha",
            "loom",
            "loomalpha",
            "review_target",
            "reviewtarget",
        ] {
            assert!(!matches_tmp_worktree_name(name), "expected {name} to miss");
        }
    }

    #[test]
    fn tmp_worktree_marker_is_required() {
        let temp = tempfile::TempDir::new().unwrap();
        let candidate = temp.path().join("rclean-candidate");
        std::fs::create_dir(&candidate).unwrap();

        assert!(
            classify(temp.path(), "rclean-candidate", &candidate).is_none(),
            "missing marker must not classify"
        );

        std::fs::write(candidate.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let draft =
            classify(temp.path(), "rclean-candidate", &candidate).expect("marker should classify");
        assert_eq!(draft.rule_id, RULE_ID);
        assert_eq!(draft.safety, Safety::Caution);
    }
}
