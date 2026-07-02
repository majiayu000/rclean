use std::path::Path;

use globset::GlobMatcher;
use serde::Deserialize;
use tracing::warn;

use crate::error::UserRuleError;
use crate::model::{CandidateDraft, Category, Safety};

/// On-disk schema for `.rclean.toml`. Kept as a private newtype so we can
/// re-shape internal representation without breaking the file format.
#[derive(Debug, Default, Deserialize)]
struct UserRuleFile {
    #[serde(default)]
    rule: Vec<UserRuleRaw>,
    /// Days after which a candidate counts as stale for ranking.
    #[serde(default)]
    stale_after_days: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct UserRuleRaw {
    id: String,
    name_glob: String,
    #[serde(default)]
    parent_markers: Vec<String>,
    category: String,
    #[serde(default = "default_safety")]
    safety: String,
    #[serde(default)]
    why: Option<String>,
    #[serde(default)]
    restore_hint: Option<String>,
}

fn default_safety() -> String {
    "safe".to_string()
}

#[derive(Debug, Clone)]
pub struct UserRule {
    id: String,
    matcher: GlobMatcher,
    parent_markers: Vec<String>,
    category: Category,
    safety: Safety,
    why: String,
    restore_hint: String,
}

#[derive(Debug, Default, Clone)]
pub struct UserRuleSet {
    rules: Vec<UserRule>,
    stale_after_days: Option<u64>,
}

impl UserRuleSet {
    /// Loads `.rclean.toml` at the scan root. A missing file is the
    /// normal case (zero user rules). Read or parse failures emit a
    /// warning and return an empty set — they don't fail the scan.
    pub fn load_from_root(root: &Path) -> Self {
        let path = root.join(".rclean.toml");
        if !path.is_file() {
            return Self::default();
        }
        let raw = match std::fs::read_to_string(&path) {
            Ok(r) => r,
            Err(err) => {
                warn!("warning: failed to read {}: {err}", path.display());
                return Self::default();
            }
        };
        let parsed: UserRuleFile = match toml::from_str(&raw) {
            Ok(p) => p,
            Err(err) => {
                warn!("warning: invalid {}: {err}", path.display());
                return Self::default();
            }
        };
        let mut rules: Vec<UserRule> = Vec::new();
        for raw in parsed.rule {
            match validate(raw) {
                Ok(rule) => {
                    if rules.iter().any(|existing| existing.id == rule.id) {
                        // First-match wins (declaration order). Warn so
                        // the user knows the duplicate later rule will
                        // never fire.
                        warn!(
                            "warning: .rclean.toml duplicate rule id '{}' - keeping the first, ignoring the rest",
                            rule.id
                        );
                        continue;
                    }
                    rules.push(rule);
                }
                Err(err) => warn!("warning: .rclean.toml rule rejected: {err}"),
            }
        }
        Self {
            rules,
            stale_after_days: parsed.stale_after_days,
        }
    }

    /// `stale_after_days` from `.rclean.toml`, if the file set one.
    pub fn stale_after_days(&self) -> Option<u64> {
        self.stale_after_days
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Try every user rule in declaration order. Returns the first match.
    /// `project_dir` is the directory containing the candidate (used to
    /// resolve `parent_markers`).
    ///
    /// `parent_markers` entries are matched with `Path::exists()`, so
    /// both file markers (e.g. `Makefile`) and directory markers
    /// (e.g. `.git`, `vendor/`) are honored. Multiple markers behave
    /// as OR — any one existing is enough.
    pub fn classify(&self, dir_name: &str, project_dir: &Path) -> Option<CandidateDraft> {
        for rule in &self.rules {
            if !rule.matcher.is_match(dir_name) {
                continue;
            }
            if !rule.parent_markers.is_empty()
                && !rule
                    .parent_markers
                    .iter()
                    .any(|marker| project_dir.join(marker).exists())
            {
                continue;
            }
            return Some(CandidateDraft {
                path: project_dir.join(dir_name),
                name: dir_name.to_string(),
                rule_id: rule.id.clone(),
                category: rule.category,
                safety: rule.safety,
                reasons: vec![rule.why.clone()],
                warnings: Vec::new(),
                restore_hint: rule.restore_hint.clone(),
            });
        }
        None
    }
}

fn validate(raw: UserRuleRaw) -> Result<UserRule, UserRuleError> {
    if raw.id.is_empty() {
        return Err(UserRuleError::EmptyId);
    }
    let category: Category =
        raw.category
            .parse()
            .map_err(|message: String| UserRuleError::InvalidCategory {
                id: raw.id.clone(),
                message,
            })?;
    let safety = parse_safety(&raw.safety, &raw.id)?;
    if raw.parent_markers.is_empty() && safety == Safety::Caution {
        // A bare-name caution rule with no parent_markers can fire under
        // any directory, including system locations. Reject as part of
        // SPEC §4.2 ("user rule with safety=caution requires at least
        // one parent_markers entry").
        return Err(UserRuleError::CautionRequiresParentMarkers { id: raw.id.clone() });
    }
    let glob = globset::Glob::new(&raw.name_glob).map_err(|source| UserRuleError::InvalidGlob {
        id: raw.id.clone(),
        source,
    })?;
    let matcher = glob.compile_matcher();
    Ok(UserRule {
        id: raw.id.clone(),
        matcher,
        parent_markers: raw.parent_markers,
        category,
        safety,
        why: raw
            .why
            .unwrap_or_else(|| format!("matches user rule '{}'", raw.id)),
        restore_hint: raw.restore_hint.unwrap_or_default(),
    })
}

fn parse_safety(s: &str, id: &str) -> Result<Safety, UserRuleError> {
    match s {
        "safe" => Ok(Safety::Safe),
        "caution" => Ok(Safety::Caution),
        "blocked" => Err(UserRuleError::BlockedSafety { id: id.to_string() }),
        other => Err(UserRuleError::InvalidSafety {
            id: id.to_string(),
            safety: other.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn missing_file_yields_empty_set() {
        let temp = TempDir::new().unwrap();
        let set = UserRuleSet::load_from_root(temp.path());
        assert!(set.is_empty());
    }

    #[test]
    fn parses_valid_rule_and_matches() {
        let temp = TempDir::new().unwrap();
        std::fs::write(
            temp.path().join(".rclean.toml"),
            r#"
[[rule]]
id = "user.my_build"
name_glob = "my_build_*"
parent_markers = ["Makefile"]
category = "build"
safety = "safe"
why = "Custom Makefile build dir"
restore_hint = "make build"
"#,
        )
        .unwrap();
        std::fs::write(temp.path().join("Makefile"), "all:\n").unwrap();

        let set = UserRuleSet::load_from_root(temp.path());
        let draft = set.classify("my_build_x86", temp.path()).unwrap();
        assert_eq!(draft.rule_id, "user.my_build");
        assert_eq!(draft.safety, Safety::Safe);
        assert_eq!(draft.category, Category::Build);
    }

    #[test]
    fn rule_without_parent_marker_does_not_fire_when_marker_missing() {
        let temp = TempDir::new().unwrap();
        std::fs::write(
            temp.path().join(".rclean.toml"),
            r#"
[[rule]]
id = "user.my_build"
name_glob = "my_build_*"
parent_markers = ["Makefile"]
category = "build"
"#,
        )
        .unwrap();
        // No Makefile written.

        let set = UserRuleSet::load_from_root(temp.path());
        assert!(set.classify("my_build_x86", temp.path()).is_none());
    }

    #[test]
    fn safety_blocked_is_rejected_at_load_time() {
        let temp = TempDir::new().unwrap();
        std::fs::write(
            temp.path().join(".rclean.toml"),
            r#"
[[rule]]
id = "user.evil"
name_glob = "*"
category = "build"
safety = "blocked"
"#,
        )
        .unwrap();

        let set = UserRuleSet::load_from_root(temp.path());
        // Rule was rejected → no rules in set.
        assert!(set.is_empty());
    }

    #[test]
    fn caution_without_parent_markers_is_rejected() {
        let temp = TempDir::new().unwrap();
        std::fs::write(
            temp.path().join(".rclean.toml"),
            r#"
[[rule]]
id = "user.loose"
name_glob = "*"
category = "build"
safety = "caution"
"#,
        )
        .unwrap();

        let set = UserRuleSet::load_from_root(temp.path());
        assert!(set.is_empty());
    }

    #[test]
    fn parent_marker_accepts_directory_marker() {
        // Review on #36 flagged that `parent_markers` previously only
        // accepted file markers (`is_file()`). Directory markers like
        // `.git/` or `vendor/` are equally valid project signals and
        // should not silently mismatch.
        let temp = TempDir::new().unwrap();
        std::fs::write(
            temp.path().join(".rclean.toml"),
            r#"
[[rule]]
id = "user.git_repo_target"
name_glob = "my_build_*"
parent_markers = [".git"]
category = "build"
safety = "safe"
"#,
        )
        .unwrap();
        // .git as a DIRECTORY (the normal case).
        std::fs::create_dir(temp.path().join(".git")).unwrap();

        let set = UserRuleSet::load_from_root(temp.path());
        let draft = set
            .classify("my_build_x86", temp.path())
            .expect(".git directory marker should satisfy parent_markers");
        assert_eq!(draft.rule_id, "user.git_repo_target");
    }

    #[test]
    fn mixed_blocked_and_valid_rules_drop_only_the_blocked_one() {
        // Review on #36 flagged that the original blocked-rejection
        // test had only one rule in the TOML — there was no regression
        // protection against a future change that drops the entire
        // file when any rule is bad. This test puts a valid `safe` rule
        // *after* a blocked rule and asserts the valid one still loads.
        let temp = TempDir::new().unwrap();
        std::fs::write(
            temp.path().join(".rclean.toml"),
            r#"
[[rule]]
id = "user.evil"
name_glob = "*"
parent_markers = ["Makefile"]
category = "build"
safety = "blocked"

[[rule]]
id = "user.good"
name_glob = "my_build_*"
parent_markers = ["Makefile"]
category = "build"
safety = "safe"
"#,
        )
        .unwrap();
        std::fs::write(temp.path().join("Makefile"), "all:\n").unwrap();

        let set = UserRuleSet::load_from_root(temp.path());
        // The blocked rule was rejected but `user.good` survived.
        let draft = set
            .classify("my_build_x86", temp.path())
            .expect("valid rule following a blocked rule should still load");
        assert_eq!(draft.rule_id, "user.good");
    }

    #[test]
    fn duplicate_rule_id_keeps_first_and_warns() {
        // Review on #36 flagged that two rules with the same `id`
        // silently coexisted and the second one was unreachable due to
        // first-match dispatch. We now warn at load time and drop the
        // duplicate. Only the first declaration's settings remain.
        let temp = TempDir::new().unwrap();
        std::fs::write(
            temp.path().join(".rclean.toml"),
            r#"
[[rule]]
id = "user.shared"
name_glob = "first_*"
parent_markers = ["Makefile"]
category = "build"
safety = "safe"

[[rule]]
id = "user.shared"
name_glob = "second_*"
parent_markers = ["Makefile"]
category = "cache"
safety = "safe"
"#,
        )
        .unwrap();
        std::fs::write(temp.path().join("Makefile"), "all:\n").unwrap();

        let set = UserRuleSet::load_from_root(temp.path());
        assert!(set.classify("first_x", temp.path()).is_some());
        // Second declaration was dropped — its name_glob never matches.
        assert!(set.classify("second_x", temp.path()).is_none());
    }
}
