//! `rclean doctor` — diagnostic for which global-cache rules are
//! applicable on this machine.
//!
//! Each Phase 1 global-path rule has a canonical anchor directory
//! (`~/.cargo`, `~/.gradle`, `~/Library/Developer`, ...). Doctor
//! reports per-rule whether that anchor exists, so the user can
//! see at a glance what `rclean scan --home` will actually touch.
//!
//! No filesystem writes, no subprocess spawns. Pure dir-exists
//! checks. Safe to run on any machine, including CI.
//!
//! See `docs/specs/v0.2-developer-mole.md` §4.3.

use std::path::PathBuf;

#[derive(Debug)]
pub struct DoctorReport {
    pub entries: Vec<DoctorEntry>,
}

#[derive(Debug)]
pub struct DoctorEntry {
    pub rule_id: &'static str,
    pub anchor: PathBuf,
    pub status: Status,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Applicable,
    Skipped { reason: &'static str },
}

impl DoctorReport {
    pub fn applicable_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.status, Status::Applicable))
            .count()
    }

    pub fn total_count(&self) -> usize {
        self.entries.len()
    }
}

pub fn diagnose() -> DoctorReport {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return DoctorReport {
            entries: Vec::new(),
        };
    };

    // Cross-platform rules. Each anchor is the directory the rule's
    // classifier checks via parent_ends_with — its existence is a
    // necessary (not sufficient) condition for the rule to fire.
    let mut entries = vec![
        check_anchor(
            "cargo.registry_cache",
            home.join(".cargo").join("registry"),
            "no Cargo install detected",
        ),
        check_anchor(
            "cargo.git_db",
            home.join(".cargo").join("git"),
            "no Cargo git deps detected",
        ),
        check_anchor(
            "gradle.caches",
            home.join(".gradle"),
            "no Gradle install detected",
        ),
        check_anchor(
            "maven.local_repo",
            home.join(".m2"),
            "no Maven install detected",
        ),
        check_anchor(
            "node.npm_cacache",
            home.join(".npm"),
            "no npm install detected",
        ),
    ];

    // pip uses different anchors per platform.
    #[cfg(target_os = "macos")]
    {
        entries.push(check_anchor(
            "pip.cache",
            home.join("Library").join("Caches"),
            "no Library/Caches directory",
        ));
    }
    #[cfg(not(target_os = "macos"))]
    {
        entries.push(check_anchor(
            "pip.cache",
            home.join(".cache"),
            "no XDG cache directory",
        ));
    }

    // macOS-only rules. On non-macOS the anchor never exists, so the
    // entry is reported as Skipped with a platform reason — gives
    // Linux users an accurate "this rule doesn't apply here" instead
    // of hiding it.
    #[cfg(target_os = "macos")]
    {
        entries.push(check_anchor(
            "node.yarn_cache",
            home.join("Library").join("Caches"),
            "no Library/Caches directory",
        ));
        entries.push(check_anchor(
            "xcode.derived_data",
            home.join("Library").join("Developer").join("Xcode"),
            "no Xcode install detected",
        ));
        entries.push(check_anchor(
            "xcode.simulators",
            home.join("Library").join("Developer"),
            "no Xcode install detected",
        ));
    }
    #[cfg(not(target_os = "macos"))]
    {
        entries.push(DoctorEntry {
            rule_id: "node.yarn_cache",
            anchor: PathBuf::from("(macOS only)"),
            status: Status::Skipped {
                reason: "rule only applies on macOS",
            },
        });
        entries.push(DoctorEntry {
            rule_id: "xcode.derived_data",
            anchor: PathBuf::from("(macOS only)"),
            status: Status::Skipped {
                reason: "rule only applies on macOS",
            },
        });
        entries.push(DoctorEntry {
            rule_id: "xcode.simulators",
            anchor: PathBuf::from("(macOS only)"),
            status: Status::Skipped {
                reason: "rule only applies on macOS",
            },
        });
    }

    DoctorReport { entries }
}

fn check_anchor(rule_id: &'static str, anchor: PathBuf, missing_reason: &'static str) -> DoctorEntry {
    let exists = anchor.is_dir();
    let status = if exists {
        Status::Applicable
    } else {
        Status::Skipped {
            reason: missing_reason,
        }
    };
    DoctorEntry {
        rule_id,
        anchor,
        status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn diagnose_returns_one_entry_per_phase1_global_rule() {
        // Run with a synthetic HOME to make the result deterministic.
        let temp = tempfile::TempDir::new().unwrap();
        // SAFETY: tests in a single binary share process env. The
        // assertion only relies on the total entry count, which is
        // the same regardless of which anchors exist.
        let _restore = with_home(temp.path());

        let report = diagnose();
        // 6 cross-platform + 3 macOS-only entries (or 3 stubbed
        // skipped entries on non-macOS). Either way: 9 total,
        // matching the v0.2 Phase 1 ruleset.
        assert_eq!(report.total_count(), 9);
    }

    #[test]
    fn diagnose_marks_existing_anchor_applicable() {
        let temp = tempfile::TempDir::new().unwrap();
        // Synthesize ~/.cargo/registry so cargo.registry_cache becomes
        // applicable.
        fs::create_dir_all(temp.path().join(".cargo").join("registry")).unwrap();
        let _restore = with_home(temp.path());

        let report = diagnose();
        let cargo_reg = report
            .entries
            .iter()
            .find(|e| e.rule_id == "cargo.registry_cache")
            .expect("cargo.registry_cache entry should exist");
        assert_eq!(cargo_reg.status, Status::Applicable);
    }

    #[test]
    fn diagnose_marks_missing_anchor_skipped() {
        let temp = tempfile::TempDir::new().unwrap();
        let _restore = with_home(temp.path());

        let report = diagnose();
        let cargo_reg = report
            .entries
            .iter()
            .find(|e| e.rule_id == "cargo.registry_cache")
            .expect("cargo.registry_cache entry should exist");
        assert!(matches!(cargo_reg.status, Status::Skipped { .. }));
    }

    /// Test helper that swaps HOME for the duration of the test and
    /// restores it on drop. Avoids leaking the override into other
    /// tests in the same binary.
    fn with_home(path: &Path) -> HomeGuard {
        let previous = std::env::var_os("HOME");
        // SAFETY: tests run serially when they touch env vars; the
        // outer Drop restores HOME so other tests aren't affected.
        unsafe {
            std::env::set_var("HOME", path);
        }
        HomeGuard { previous }
    }

    struct HomeGuard {
        previous: Option<std::ffi::OsString>,
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            // SAFETY: see with_home above.
            unsafe {
                match &self.previous {
                    Some(v) => std::env::set_var("HOME", v),
                    None => std::env::remove_var("HOME"),
                }
            }
        }
    }
}
