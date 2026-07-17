use super::*;
use crate::model::{ActivityInfo, Candidate, Category, ProjectReport, Summary};
use tempfile::TempDir;

fn candidate_map(project: &str, bytes: u64) -> CandidateMap {
    BTreeMap::from([(
        format!("{project}/target"),
        CandidateSnapshot {
            bytes,
            safety: Safety::Safe,
        },
    )])
}

fn report_with_projects(projects: &[(&str, u64)]) -> ScanReport {
    ScanReport {
        schema_version: 1,
        tool_version: "test".to_string(),
        scanned_at: "2026-07-16T00:00:00Z".to_string(),
        roots: vec!["/workspace".to_string()],
        disk_attribution: None,
        warnings: Vec::new(),
        stale_after_days: 30,
        summary: Summary::default(),
        projects: projects
            .iter()
            .map(|(path, bytes)| ProjectReport {
                path: (*path).to_string(),
                kind: "Rust".to_string(),
                markers: vec!["Cargo.toml".to_string()],
                git: None,
                activity: ActivityInfo {
                    last_modified: "2026-07-16T00:00:00Z".to_string(),
                    source: "test".to_string(),
                },
                candidates: vec![Candidate {
                    path: format!("{path}/target"),
                    name: "target".to_string(),
                    rule_id: "rust.target".to_string(),
                    category: Category::Build,
                    bytes: *bytes,
                    safety: Safety::Safe,
                    requires_sudo: false,
                    reasons: vec!["test fixture".to_string()],
                    warnings: Vec::new(),
                    restore_hint: "cargo build".to_string(),
                    risk_score: 0.0,
                    staleness_days: Some(1),
                }],
                total_bytes: *bytes,
                project_bytes: *bytes,
                artifact_percent: 100.0,
            })
            .collect(),
    }
}

#[test]
fn timestamped_plan_path_preserves_existing_files_and_uses_numeric_suffixes() {
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("auto.json");
    let stamp = "20260716T192240Z";
    let first = temp.path().join("auto-20260716T192240Z.json");

    assert_eq!(next_timestamped_path(&base, stamp).unwrap(), first);

    std::fs::write(&first, b"sentinel").unwrap();
    let second = temp.path().join("auto-20260716T192240Z-2.json");
    assert_eq!(next_timestamped_path(&base, stamp).unwrap(), second);
    assert_eq!(std::fs::read(&first).unwrap(), b"sentinel");

    std::fs::write(&second, b"second sentinel").unwrap();
    assert_eq!(
        next_timestamped_path(&base, stamp).unwrap(),
        temp.path().join("auto-20260716T192240Z-3.json")
    );
    assert_eq!(std::fs::read(&first).unwrap(), b"sentinel");
    assert_eq!(std::fs::read(&second).unwrap(), b"second sentinel");
}

#[test]
fn timestamped_plan_path_preserves_extensionless_names() {
    assert_eq!(
        timestamped_path(Path::new("auto"), "20260716T192240Z", None),
        PathBuf::from("auto-20260716T192240Z")
    );
    assert_eq!(
        timestamped_path(Path::new("auto"), "20260716T192240Z", Some(2)),
        PathBuf::from("auto-20260716T192240Z-2")
    );
}

#[cfg(unix)]
#[test]
fn timestamped_plan_path_preserves_non_utf8_stem_fallback() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let base = PathBuf::from(OsString::from_vec(vec![0xff, b'.', b'j', b's', b'o', b'n']));
    assert_eq!(
        timestamped_path(&base, "20260716T192240Z", None),
        PathBuf::from("rclean-watch-20260716T192240Z.json")
    );
}

#[test]
fn timestamped_plan_path_surfaces_probe_errors() {
    let error = next_timestamped_path_with(Path::new("auto.json"), "20260716T192240Z", 1, |_| {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "probe denied",
        ))
    })
    .unwrap_err();

    match error {
        PlanError::Io { path, source } => {
            assert_eq!(path, PathBuf::from("auto-20260716T192240Z.json"));
            assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
        }
        other => panic!("expected PlanError::Io, got {other}"),
    }
}

#[test]
fn timestamped_plan_path_surfaces_sequence_exhaustion() {
    let error =
        next_timestamped_path_with(Path::new("auto.json"), "20260716T192240Z", u64::MAX, |_| {
            Ok(true)
        })
        .unwrap_err();

    assert!(matches!(error, PlanError::Generic(message) if message.contains("exhausted")));
}

#[test]
fn maps_lockfile_to_project_root() {
    assert_eq!(
        project_root_for_lockfile(Path::new("/repo/app/package-lock.json")).unwrap(),
        PathBuf::from("/repo/app")
    );
    assert_eq!(
        project_root_for_lockfile(Path::new("/repo/app/.git/HEAD")).unwrap(),
        PathBuf::from("/repo/app")
    );
    assert!(project_root_for_lockfile(Path::new("/repo/app/package.json")).is_none());
}

#[test]
fn reconciles_missing_projects_in_non_empty_polling_scope() {
    let mut state = WatchState {
        by_project: BTreeMap::from([
            ("/workspace/a".to_string(), candidate_map("/workspace/a", 1)),
            ("/workspace/b".to_string(), candidate_map("/workspace/b", 2)),
            ("/other/c".to_string(), candidate_map("/other/c", 3)),
        ]),
    };
    let outside_before = state.by_project["/other/c"].clone();

    assert!(
        state
            .update_project(
                Path::new("/workspace"),
                &report_with_projects(&[("/workspace/a", 4)]),
            )
            .is_ok()
    );

    assert_eq!(
        state.by_project["/workspace/a"]["/workspace/a/target"].bytes,
        4
    );
    assert!(!state.by_project.contains_key("/workspace/b"));
    assert!(state.by_project["/other/c"] == outside_before);
}

#[test]
fn empty_refresh_removes_descendants_without_string_prefix_collisions() {
    let mut state = WatchState {
        by_project: BTreeMap::from([
            ("/workspace".to_string(), candidate_map("/workspace", 1)),
            ("/workspace/a".to_string(), candidate_map("/workspace/a", 2)),
            ("/workspace/b".to_string(), candidate_map("/workspace/b", 3)),
            (
                "/workspace-ab".to_string(),
                candidate_map("/workspace-ab", 4),
            ),
        ]),
    };

    assert!(
        state
            .update_project(Path::new("/workspace"), &report_with_projects(&[]))
            .is_ok()
    );

    assert!(!state.by_project.contains_key("/workspace"));
    assert!(!state.by_project.contains_key("/workspace/a"));
    assert!(!state.by_project.contains_key("/workspace/b"));
    assert!(state.by_project.contains_key("/workspace-ab"));
}

#[test]
fn single_project_refresh_preserves_sibling_state() {
    let mut state = WatchState {
        by_project: BTreeMap::from([
            ("/workspace/a".to_string(), candidate_map("/workspace/a", 1)),
            ("/workspace/b".to_string(), candidate_map("/workspace/b", 2)),
            (
                "/workspace/ab".to_string(),
                candidate_map("/workspace/ab", 4),
            ),
        ]),
    };
    let sibling_before = state.by_project["/workspace/b"].clone();
    let prefix_sibling_before = state.by_project["/workspace/ab"].clone();

    assert!(
        state
            .update_project(
                Path::new("/workspace/a"),
                &report_with_projects(&[("/workspace/a", 3)]),
            )
            .is_ok()
    );

    assert!(state.by_project["/workspace/b"] == sibling_before);
    assert!(state.by_project["/workspace/ab"] == prefix_sibling_before);
}
