use std::cell::{Cell, OnceCell};
use std::fs::{self, FileTimes, OpenOptions};

use tempfile::TempDir;

use super::*;

#[test]
fn project_risk_cache_computes_once_and_reuses_value() {
    let cache = OnceCell::new();
    let calls = Cell::new(0);

    assert!(cache.get().is_none());
    assert_eq!(calls.get(), 0);

    let first = cached_project_risk_score(&cache, || {
        calls.set(calls.get() + 1);
        0.4
    });
    let second = cached_project_risk_score(&cache, || {
        calls.set(calls.get() + 1);
        panic!("cached risk score must not be recomputed");
    });

    assert_eq!(first, 0.4);
    assert_eq!(second, first);
    assert_eq!(calls.get(), 1);
}

fn write_with_modified(path: &Path, modified: SystemTime) {
    fs::write(path, b"source").unwrap();
    OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(FileTimes::new().set_modified(modified))
        .unwrap();
}

#[test]
fn project_activities_handles_empty_and_single_inputs() {
    assert!(project_activities(&[], 6).is_empty());

    let temp = TempDir::new().unwrap();
    let project = temp.path().join("single");
    fs::create_dir(&project).unwrap();
    let expected = SystemTime::UNIX_EPOCH + Duration::from_secs(4_000_000_000);
    write_with_modified(&project.join("source.rs"), expected);

    assert_eq!(project_activities(&[project], 6), vec![expected]);
}

#[test]
fn project_activities_preserves_input_order() {
    let temp = TempDir::new().unwrap();
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    fs::create_dir(&first).unwrap();
    fs::create_dir(&second).unwrap();

    let first_time = SystemTime::UNIX_EPOCH + Duration::from_secs(4_000_000_000);
    let second_time = first_time + Duration::from_secs(60);
    write_with_modified(&first.join("source.rs"), first_time);
    write_with_modified(&second.join("source.rs"), second_time);

    assert_eq!(
        project_activities(&[second, first], 6),
        vec![second_time, first_time]
    );
}

#[test]
fn project_activities_match_serial_traversal_boundaries() {
    let temp = TempDir::new().unwrap();
    let mut projects = Vec::new();

    for name in ["alpha", "beta"] {
        let project = temp.path().join(name);
        fs::create_dir_all(project.join("src").join("deep")).unwrap();
        fs::create_dir(project.join("node_modules")).unwrap();
        fs::create_dir(project.join(".git")).unwrap();
        fs::write(project.join("src").join("visible.rs"), b"visible").unwrap();
        fs::write(
            project.join("src").join("deep").join("too_deep.rs"),
            b"deep",
        )
        .unwrap();
        fs::write(project.join("node_modules").join("artifact"), b"artifact").unwrap();
        fs::write(project.join(".git").join("index"), b"git").unwrap();
        let external = temp.path().join(format!("{name}-external.rs"));
        fs::write(&external, b"external").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&external, project.join("linked-source.rs")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&external, project.join("linked-source.rs")).unwrap();
        projects.push(project);
    }

    let serial = projects
        .iter()
        .map(|project| project_activity(project, 2).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(project_activities(&projects, 2), serial);
}

#[test]
fn project_activities_preserves_missing_project_slots() {
    let temp = TempDir::new().unwrap();
    let before = SystemTime::now();
    let missing = vec![
        temp.path().join("missing-project-a"),
        temp.path().join("missing-project-b"),
    ];

    let activities = project_activities(&missing, 6);
    let after = SystemTime::now();

    assert_eq!(activities.len(), missing.len());
    assert!(
        activities
            .iter()
            .all(|activity| *activity >= before && *activity <= after)
    );
}

/// End-to-end for #354: a candidate whose own content is old must report
/// its true age even when the surrounding project activity is fresh
/// (the situation a busy shared parent like `~/Library/Caches` creates).
#[test]
fn staleness_reflects_candidate_age_not_a_fresh_project_activity() {
    use crate::model::{Category, Safety};
    use crate::scan::sizer::SourceSizeIndex;
    use std::time::Duration;

    let temp = TempDir::new().unwrap();
    let cache = temp.path().join("node_modules");
    fs::create_dir(&cache).unwrap();
    let old = SystemTime::now() - Duration::from_secs(40 * 86_400);
    write_with_modified(&cache.join("blob"), old);

    let drafts = vec![CandidateDraft {
        path: cache.clone(),
        name: "node_modules".to_string(),
        rule_id: "node.node_modules".to_string(),
        category: Category::Deps,
        safety: Safety::Safe,
        reasons: Vec::new(),
        warnings: Vec::new(),
        restore_hint: "npm install".to_string(),
    }];

    let options = ScanOptions {
        min_size: 0,
        ..ScanOptions::default()
    };
    let source_sizes = SourceSizeIndex::from_dir_sizes(&Default::default());

    // Project activity is "now" — as if an unrelated sibling under the
    // same parent was just touched. Pre-#354 this made the candidate
    // read 0d; it must now report its own ~40-day age.
    let (report, _warnings) = build_project_report(
        temp.path(),
        temp.path(),
        drafts,
        &options,
        &GitCache::new(),
        &source_sizes,
        SystemTime::now(),
    )
    .unwrap();

    let candidate = &report.candidates[0];
    assert_eq!(
        candidate.staleness_days,
        Some(40),
        "candidate must report its own 40-day age, not the fresh project activity"
    );
}
