use super::*;
use crate::test_support::{EnvGuard, with_env_vars};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[test]
fn diagnose_matches_global_rule_catalog_exactly() {
    // Run with a synthetic HOME to make the result deterministic.
    let temp = tempfile::TempDir::new().unwrap();
    // SAFETY: tests in a single binary share process env. The
    // assertions only rely on rule IDs, which are the same regardless
    // of which anchors exist.
    let _restore = with_home(temp.path());

    let report = diagnose();
    let actual = report
        .entries
        .iter()
        .map(|entry| entry.rule_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        report.entries.len(),
        actual.len(),
        "doctor emitted duplicate rule ids: {:?}",
        report
            .entries
            .iter()
            .map(|entry| entry.rule_id)
            .collect::<Vec<_>>()
    );

    let expected = crate::rules::rule_catalog()
        .into_iter()
        .filter(|rule| crate::rules::is_global_rule(rule.rule_id))
        .map(|rule| rule.rule_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "doctor and global rule catalog drifted");
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
///
/// Backed by the crate-wide lock in `test_support` rather than a
/// module-local one: other modules override the same variables, and a
/// per-module mutex would not serialize against them.
fn with_home(path: &Path) -> EnvGuard {
    with_env_vars(&[("HOME", Some(&path.display().to_string()))])
}
