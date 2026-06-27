use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::schema::PlanCandidate;
use super::*;
use crate::model::{ActivityInfo, Candidate, Category, ProjectReport, Safety, ScanReport, Summary};

fn report(root: &Path, candidate_path: &Path) -> ScanReport {
    ScanReport {
        schema_version: 1,
        tool_version: "0.1.0".to_string(),
        scanned_at: "2026-05-06T00:00:00Z".to_string(),
        roots: vec![root.display().to_string()],
        disk_attribution: None,
        warnings: Vec::new(),
        summary: Summary {
            projects_scanned: 1,
            projects_with_candidates: 1,
            candidates: 1,
            safe_candidates: 1,
            caution_candidates: 0,
            blocked_candidates: 0,
            report_only_candidates: 0,
            total_bytes: 3,
        },
        projects: vec![ProjectReport {
            path: root.display().to_string(),
            kind: "Node.js".to_string(),
            markers: vec!["package.json".to_string()],
            git: None,
            activity: ActivityInfo {
                last_modified: "2026-05-06T00:00:00Z".to_string(),
                source: "test".to_string(),
            },
            candidates: vec![Candidate {
                path: candidate_path.display().to_string(),
                name: "node_modules".to_string(),
                rule_id: "node.node_modules".to_string(),
                category: Category::Deps,
                bytes: 3,
                safety: Safety::Safe,
                requires_sudo: false,
                reasons: vec!["test".to_string()],
                warnings: Vec::new(),
                restore_hint: "install".to_string(),
                risk_score: 0.42,
            }],
            total_bytes: 3,
            project_bytes: 5,
            artifact_percent: 60.0,
        }],
    }
}

fn create_node_project(root: &Path) -> PathBuf {
    let candidate = root.join("node_modules");
    fs::create_dir(&candidate).unwrap();
    fs::write(root.join("package.json"), "{}").unwrap();
    candidate
}

fn create_docker_storage_target(root: &Path) -> PathBuf {
    let project = root.join("var").join("lib").join("docker").join("project");
    let candidate = project.join("target");
    fs::create_dir_all(&candidate).unwrap();
    fs::write(project.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
    fs::write(candidate.join("placeholder"), b"x").unwrap();
    candidate
}

#[test]
fn writes_and_revalidates_plan() {
    let temp = TempDir::new().unwrap();
    let candidate = create_node_project(temp.path());
    let plan_path = temp.path().join("plan.json");
    let report = report(temp.path(), &candidate);

    write_action_plan(&report, &plan_path, false, false, "trash").unwrap();
    let plan = read_action_plan(&plan_path).unwrap();
    let selected = selected_from_action_plan(&plan).unwrap();

    assert_eq!(selected.len(), 1);
    revalidate_selected(&plan, &selected).unwrap();
}

#[test]
fn writes_schema_v2_candidate_id_and_risk_score() {
    let temp = TempDir::new().unwrap();
    let candidate = create_node_project(temp.path());
    let plan_path = temp.path().join("plan.json");
    let report = report(temp.path(), &candidate);

    write_action_plan(&report, &plan_path, false, false, "trash").unwrap();
    let plan = read_action_plan(&plan_path).unwrap();

    assert_eq!(plan.schema_version, ACTION_PLAN_SCHEMA_VERSION);
    assert_eq!(plan.selected.len(), 1);
    assert_eq!(plan.selected[0].id.len(), 26);
    assert!(
        plan.selected[0]
            .id
            .chars()
            .all(|c| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c)),
        "candidate id should use Crockford ULID alphabet"
    );
    assert!((plan.selected[0].risk_score - 0.42).abs() < f32::EPSILON);
    assert!(!plan.selected[0].requires_sudo);
}

#[test]
fn old_plan_candidate_missing_requires_sudo_defaults_false() {
    let temp = TempDir::new().unwrap();
    let plan_path = temp.path().join("plan.json");
    fs::write(
        &plan_path,
        r#"{
            "schemaVersion": 2,
            "toolVersion": "0.1.0",
            "generatedAt": "2026-05-06T00:00:00Z",
            "deleteMode": "trash",
            "roots": [],
            "summary": {
                "projectsScanned": 0,
                "projectsWithCandidates": 0,
                "candidates": 1,
                "safeCandidates": 1,
                "cautionCandidates": 0,
                "blockedCandidates": 0,
                "totalBytes": 3
            },
            "selected": [{
                "id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
                "path": "/tmp/project/node_modules",
                "ruleId": "node.node_modules",
                "bytes": 3,
                "safety": "safe",
                "category": "deps",
                "riskScore": 0.0
            }],
            "projects": []
        }"#,
    )
    .unwrap();

    let plan = read_action_plan(&plan_path).unwrap();

    assert!(!plan.selected[0].requires_sudo);
}

#[test]
fn plan_candidate_serializes_requires_sudo_as_camel_case_when_true() {
    let candidate = PlanCandidate {
        id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
        path: "/Library/Application Support/com.apple.idleassetsd".to_string(),
        rule_id: "apple.idleassetsd".to_string(),
        bytes: 1,
        safety: Safety::ReportOnly,
        requires_sudo: true,
        category: Category::Cache,
        risk_score: 0.0,
    };

    let value = serde_json::to_value(candidate).unwrap();

    assert_eq!(value["requiresSudo"], true);
}

#[test]
fn selected_from_action_plan_refuses_requires_sudo_candidate() {
    let temp = TempDir::new().unwrap();
    let plan = ActionPlan {
        schema_version: ACTION_PLAN_SCHEMA_VERSION,
        tool_version: "0.1.0".to_string(),
        generated_at: "2026-05-06T00:00:00Z".to_string(),
        delete_mode: "trash".to_string(),
        roots: vec![temp.path().display().to_string()],
        summary: Summary::default(),
        selected: vec![PlanCandidate {
            id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            path: temp
                .path()
                .join("Library")
                .join("Application Support")
                .join("com.apple.idleassetsd")
                .display()
                .to_string(),
            rule_id: "apple.idleassetsd".to_string(),
            bytes: 1,
            safety: Safety::ReportOnly,
            requires_sudo: true,
            category: Category::Cache,
            risk_score: 0.0,
        }],
        projects: Vec::new(),
    };

    let err = selected_from_action_plan(&plan)
        .expect_err("requires-sudo plan candidates must be refused")
        .to_string();

    assert!(err.contains("requires administrator access"));
    assert!(err.contains("will not run sudo"));
}

#[test]
fn revalidation_rejects_stale_plan_path() {
    let temp = TempDir::new().unwrap();
    let candidate = create_node_project(temp.path());
    let plan_path = temp.path().join("plan.json");
    let report = report(temp.path(), &candidate);

    write_action_plan(&report, &plan_path, false, false, "trash").unwrap();
    let plan = read_action_plan(&plan_path).unwrap();
    let selected = selected_from_action_plan(&plan).unwrap();
    fs::remove_dir_all(&candidate).unwrap();

    assert!(revalidate_selected(&plan, &selected).is_err());
}

#[test]
fn revalidation_rejects_symlinked_plan_path() {
    let temp = TempDir::new().unwrap();
    let candidate = create_node_project(temp.path());
    let real = temp.path().join("real_modules");
    fs::create_dir(&real).unwrap();
    let plan_path = temp.path().join("plan.json");
    let report = report(temp.path(), &candidate);

    write_action_plan(&report, &plan_path, false, false, "trash").unwrap();
    let plan = read_action_plan(&plan_path).unwrap();
    let selected = selected_from_action_plan(&plan).unwrap();
    fs::remove_dir_all(&candidate).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real, &candidate).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&real, &candidate).unwrap();

    assert!(revalidate_selected(&plan, &selected).is_err());
}

#[test]
#[cfg(unix)]
fn revalidation_rejects_hardlinked_plan_path() {
    let temp = TempDir::new().unwrap();
    let candidate = create_node_project(temp.path());
    let original = temp.path().join("original");
    let plan_path = temp.path().join("plan.json");
    let report = report(temp.path(), &candidate);

    write_action_plan(&report, &plan_path, false, false, "trash").unwrap();
    let plan = read_action_plan(&plan_path).unwrap();
    let selected = selected_from_action_plan(&plan).unwrap();
    fs::remove_dir_all(&candidate).unwrap();
    fs::write(&original, "content").unwrap();
    fs::hard_link(&original, &candidate).unwrap();

    let err = revalidate_selected(&plan, &selected)
        .expect_err("hardlinked plan path must be rejected")
        .to_string();

    assert!(err.contains("hardlinked file"), "unexpected error: {err}");
}

#[test]
fn tampered_plan_with_unrecognized_path_is_rejected() {
    let temp = TempDir::new().unwrap();
    let candidate = temp.path().join("not_a_real_artifact");
    fs::create_dir(&candidate).unwrap();
    let plan_path = temp.path().join("plan.json");
    let mut report = report(temp.path(), &candidate);
    report.projects[0].candidates[0].name = "not_a_real_artifact".to_string();
    report.projects[0].candidates[0].rule_id = "fake.rule".to_string();
    report.projects[0].candidates[0].path = candidate.display().to_string();

    write_action_plan(&report, &plan_path, false, false, "trash").unwrap();
    let plan = read_action_plan(&plan_path).unwrap();
    let err = selected_from_action_plan(&plan)
        .expect_err("should reject unrecognized paths")
        .to_string();
    assert!(
        err.contains("not recognized") || err.contains("rule"),
        "unexpected error: {err}"
    );
}

#[test]
fn tampered_plan_promoting_blocked_to_safe_is_rejected() {
    let temp = TempDir::new().unwrap();
    let candidate = create_node_project(temp.path());
    let plan_path = temp.path().join("plan.json");
    let report = report(temp.path(), &candidate);

    write_action_plan(&report, &plan_path, false, false, "trash").unwrap();

    let raw = fs::read_to_string(&plan_path).unwrap();
    let mut plan: ActionPlan = serde_json::from_str(&raw).unwrap();
    plan.selected[0].path = "/usr/local/lib/node_modules".to_string();
    plan.selected[0].safety = Safety::Safe;
    let tampered_json = serde_json::to_string_pretty(&plan).unwrap();
    fs::write(&plan_path, tampered_json).unwrap();

    let plan = read_action_plan(&plan_path).unwrap();
    let err = selected_from_action_plan(&plan)
        .expect_err("should reject a tampered plan that injects a system path")
        .to_string();
    assert!(
        err.contains("/usr/local/lib/node_modules") || err.contains("not recognized"),
        "unexpected error: {err}"
    );
}

#[test]
fn tampered_plan_pointing_at_codex_sessions_is_rejected() {
    let temp = TempDir::new().unwrap();
    let candidate = create_node_project(temp.path());
    let sessions = temp.path().join(".codex").join("sessions");
    fs::create_dir_all(&sessions).unwrap();
    let plan_path = temp.path().join("plan.json");
    let report = report(temp.path(), &candidate);

    write_action_plan(&report, &plan_path, false, false, "trash").unwrap();

    let raw = fs::read_to_string(&plan_path).unwrap();
    let mut plan: ActionPlan = serde_json::from_str(&raw).unwrap();
    plan.selected[0].path = sessions.display().to_string();
    let tampered_json = serde_json::to_string_pretty(&plan).unwrap();
    fs::write(&plan_path, tampered_json).unwrap();

    let plan = read_action_plan(&plan_path).unwrap();
    let err = selected_from_action_plan(&plan)
        .expect_err("Codex session history must never be selected from a plan")
        .to_string();

    assert!(
        err.contains("protected user data"),
        "unexpected error: {err}"
    );
}

#[test]
fn tampered_plan_pointing_at_docker_storage_is_rejected() {
    let temp = TempDir::new().unwrap();
    let candidate = create_node_project(temp.path());
    let docker_target = create_docker_storage_target(temp.path());
    let plan_path = temp.path().join("plan.json");
    let report = report(temp.path(), &candidate);

    write_action_plan(&report, &plan_path, false, false, "trash").unwrap();

    let raw = fs::read_to_string(&plan_path).unwrap();
    let mut plan: ActionPlan = serde_json::from_str(&raw).unwrap();
    plan.selected[0].path = docker_target.display().to_string();
    plan.selected[0].safety = Safety::Safe;
    let tampered_json = serde_json::to_string_pretty(&plan).unwrap();
    fs::write(&plan_path, tampered_json).unwrap();

    let plan = read_action_plan(&plan_path).unwrap();
    let err = selected_from_action_plan(&plan)
        .expect_err("Docker daemon storage must never be selected from a plan")
        .to_string();

    assert!(
        err.contains("Docker daemon storage"),
        "unexpected error: {err}"
    );
}

#[test]
fn revalidate_selected_rejects_docker_storage_path() {
    let temp = TempDir::new().unwrap();
    let candidate = create_node_project(temp.path());
    let docker_target = create_docker_storage_target(temp.path());
    let plan_path = temp.path().join("plan.json");
    let report = report(temp.path(), &candidate);

    write_action_plan(&report, &plan_path, false, false, "trash").unwrap();
    let plan = read_action_plan(&plan_path).unwrap();
    let selected = vec![crate::clean::SelectedCandidate {
        id: Some("docker-storage-test".to_string()),
        path: docker_target,
        bytes: 1,
        rule_id: "rust.target".to_string(),
        category: Category::Build,
        safety: Safety::Safe,
        requires_sudo: false,
        risk_score: 0.0,
    }];

    let err = revalidate_selected(&plan, &selected)
        .expect_err("Docker daemon storage must fail replay revalidation")
        .to_string();

    assert!(
        err.contains("Docker daemon storage"),
        "unexpected error: {err}"
    );
}

#[test]
fn tampered_plan_pointing_at_claude_user_records_is_rejected() {
    for protected in [
        ".claude/projects/encoded-path/abc.jsonl",
        ".claude/sessions/2026-05-24",
        ".claude/history.jsonl",
        ".claude/shell-snapshots/snap.json",
        ".claude/file-history/foo.diff",
        ".claude/todos/today.md",
    ] {
        let temp = TempDir::new().unwrap();
        let candidate = create_node_project(temp.path());
        let target = temp.path().join(protected);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&target, "x").unwrap();
        let plan_path = temp.path().join("plan.json");
        let report = report(temp.path(), &candidate);

        write_action_plan(&report, &plan_path, false, false, "trash").unwrap();

        let raw = fs::read_to_string(&plan_path).unwrap();
        let mut plan: ActionPlan = serde_json::from_str(&raw).unwrap();
        plan.selected[0].path = target.display().to_string();
        let tampered_json = serde_json::to_string_pretty(&plan).unwrap();
        fs::write(&plan_path, tampered_json).unwrap();

        let plan = read_action_plan(&plan_path).unwrap();
        let err = selected_from_action_plan(&plan).unwrap_err().to_string();

        assert!(
            err.contains("protected user data"),
            "expected {protected} to be rejected with protected-user-data error, got: {err}"
        );
    }
}

#[test]
fn summary_reflects_selected_not_full_scan() {
    let temp = TempDir::new().unwrap();
    let candidate = create_node_project(temp.path());
    let plan_path = temp.path().join("plan.json");
    // Scan summary claims 5 candidates / 1000 bytes; selected has 1 / 3 bytes.
    let mut report = report(temp.path(), &candidate);
    report.summary.candidates = 5;
    report.summary.safe_candidates = 4;
    report.summary.caution_candidates = 1;
    report.summary.total_bytes = 1000;

    write_action_plan(&report, &plan_path, false, false, "trash").unwrap();
    let plan = read_action_plan(&plan_path).unwrap();

    assert_eq!(plan.selected.len(), 1);
    assert_eq!(plan.summary.candidates, 1);
    assert_eq!(plan.summary.safe_candidates, 1);
    assert_eq!(plan.summary.caution_candidates, 0);
    assert_eq!(plan.summary.total_bytes, 3);
    // Scan-wide accounting still preserved.
    assert_eq!(plan.summary.projects_scanned, 1);
    assert_eq!(plan.summary.projects_with_candidates, 1);
}

#[test]
fn write_is_atomic_against_existing_file() {
    let temp = TempDir::new().unwrap();
    let candidate = create_node_project(temp.path());
    let plan_path = temp.path().join("plan.json");
    let report = report(temp.path(), &candidate);

    // Pre-existing valid plan.
    write_action_plan(&report, &plan_path, false, false, "trash").unwrap();
    let original = fs::read_to_string(&plan_path).unwrap();

    // A second write must replace atomically; no .tmp leftover under the parent.
    write_action_plan(&report, &plan_path, true, true, "permanent").unwrap();
    let after = fs::read_to_string(&plan_path).unwrap();
    assert_ne!(original, after, "second write should change content");

    let leftovers: Vec<_> = fs::read_dir(temp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name().to_string_lossy().starts_with(".tmp")
                || e.file_name().to_string_lossy().ends_with(".tmp")
        })
        .collect();
    assert!(
        leftovers.is_empty(),
        "no temp files should remain: {leftovers:?}"
    );
}

#[test]
fn rejects_plans_with_unknown_fields() {
    let temp = TempDir::new().unwrap();
    let plan_path = temp.path().join("plan.json");
    fs::write(
        &plan_path,
        r#"{
            "schemaVersion": 2,
            "toolVersion": "0.1.0",
            "generatedAt": "2026-05-06T00:00:00Z",
            "deleteMode": "trash",
            "roots": [],
            "summary": {
                "projectsScanned": 0,
                "projectsWithCandidates": 0,
                "candidates": 0,
                "safeCandidates": 0,
                "cautionCandidates": 0,
                "blockedCandidates": 0,
                "totalBytes": 0
            },
            "selected": [],
            "projects": [],
            "extraField": "should reject"
        }"#,
    )
    .unwrap();

    let err = read_action_plan(&plan_path)
        .expect_err("should reject unknown fields")
        .to_string();
    assert!(err.contains("unknown field"), "unexpected error: {err}");
}

#[test]
fn rejects_schema_v1_with_rescan_hint() {
    let temp = TempDir::new().unwrap();
    let plan_path = temp.path().join("plan.json");
    fs::write(
        &plan_path,
        r#"{
            "schemaVersion": 1,
            "toolVersion": "0.1.0",
            "generatedAt": "2026-05-06T00:00:00Z",
            "deleteMode": "trash",
            "roots": [],
            "summary": {
                "projectsScanned": 0,
                "projectsWithCandidates": 0,
                "candidates": 0,
                "safeCandidates": 0,
                "cautionCandidates": 0,
                "blockedCandidates": 0,
                "totalBytes": 0
            },
            "selected": [],
            "projects": []
        }"#,
    )
    .unwrap();

    let err = read_action_plan(&plan_path)
        .expect_err("schema v1 must be rejected")
        .to_string();
    assert!(err.contains("schema version 1"), "unexpected error: {err}");
    assert!(
        err.contains("scan --write-plan"),
        "rescan hint missing: {err}"
    );
}
