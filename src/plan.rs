use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::clean::SelectedCandidate;
use crate::error::PlanError;
use crate::model::{
    Candidate, CandidateDraft, Category, ProjectReport, Safety, ScanReport, Summary,
};
use crate::rules;
use crate::scan::is_runtime_or_system_path;
use crate::user_rules::UserRuleSet;

pub const ACTION_PLAN_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionPlan {
    pub schema_version: u32,
    pub tool_version: String,
    pub generated_at: String,
    pub delete_mode: String,
    pub roots: Vec<String>,
    pub summary: Summary,
    pub selected: Vec<PlanCandidate>,
    pub projects: Vec<ProjectReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanCandidate {
    pub id: String,
    pub path: String,
    pub rule_id: String,
    pub bytes: u64,
    pub safety: Safety,
    pub category: Category,
    pub risk_score: f32,
}

pub fn write_action_plan(
    report: &ScanReport,
    path: &Path,
    include_caution: bool,
    include_permanent: bool,
    delete_mode: &str,
) -> Result<(), PlanError> {
    let selected = collect_selected(report, include_caution);
    let summary = summarize_selected(&selected, &report.summary);
    let plan = ActionPlan {
        schema_version: ACTION_PLAN_SCHEMA_VERSION,
        tool_version: report.tool_version.clone(),
        generated_at: Utc::now().to_rfc3339(),
        delete_mode: if include_permanent {
            "permanent".to_string()
        } else {
            delete_mode.to_string()
        },
        roots: report.roots.clone(),
        summary,
        selected,
        projects: report.projects.clone(),
    };
    let json = serde_json::to_string_pretty(&plan)?;
    write_atomically(path, json.as_bytes())
}

#[cfg_attr(not(feature = "tui"), allow(dead_code))]
pub fn write_selected_action_plan(
    report: &ScanReport,
    path: &Path,
    selected: &[SelectedCandidate],
    delete_mode: &str,
) -> Result<(), PlanError> {
    let selected = collect_selected_paths(report, selected);
    let summary = summarize_selected(&selected, &report.summary);
    let plan = ActionPlan {
        schema_version: 1,
        tool_version: report.tool_version.clone(),
        generated_at: Utc::now().to_rfc3339(),
        delete_mode: delete_mode.to_string(),
        roots: report.roots.clone(),
        summary,
        selected,
        projects: report.projects.clone(),
    };
    let json = serde_json::to_string_pretty(&plan)?;
    write_atomically(path, json.as_bytes())
}

fn write_atomically(path: &Path, contents: &[u8]) -> Result<(), PlanError> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    let mut tmp = match parent {
        Some(dir) => tempfile::NamedTempFile::new_in(dir),
        None => tempfile::NamedTempFile::new_in("."),
    }
    .map_err(|source| PlanError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    use std::io::Write as _;
    tmp.write_all(contents).map_err(|source| PlanError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    tmp.as_file().sync_all().map_err(|source| PlanError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    tmp.persist(path).map_err(|err| PlanError::Io {
        path: path.to_path_buf(),
        source: err.error,
    })?;
    Ok(())
}

pub fn read_action_plan(path: &Path) -> Result<ActionPlan, PlanError> {
    let raw = fs::read_to_string(path).map_err(|source| PlanError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let value: Value = serde_json::from_str(&raw)?;
    let found_version = value
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .map(|version| version as u32)
        .unwrap_or(0);
    if found_version != ACTION_PLAN_SCHEMA_VERSION {
        return Err(PlanError::UnsupportedSchemaVersion {
            found: found_version,
            supported: ACTION_PLAN_SCHEMA_VERSION,
        });
    }
    let plan: ActionPlan = serde_json::from_value(value)?;
    validate_delete_mode(&plan.delete_mode)?;
    Ok(plan)
}

pub fn selected_from_action_plan(plan: &ActionPlan) -> Result<Vec<SelectedCandidate>, PlanError> {
    let mut selected = Vec::with_capacity(plan.selected.len());
    for candidate in &plan.selected {
        let path = PathBuf::from(&candidate.path);

        let draft = classify_plan_candidate(plan, candidate, &path).ok_or_else(|| {
            PlanError::Generic(format!(
                "{} is not recognized by any current rule (plan may be stale or tampered)",
                candidate.path
            ))
        })?;

        if is_runtime_or_system_path(&path) && !rules::is_global_rule(&draft.rule_id) {
            return Err(PlanError::Generic(format!(
                "{} is inside a protected runtime or system path; refusing to clean",
                candidate.path
            )));
        }

        if draft.safety == Safety::Blocked || draft.safety == Safety::Unknown {
            return Err(PlanError::Generic(format!(
                "{} is now classified as {:?} by rule {}; refusing to clean",
                candidate.path, draft.safety, draft.rule_id
            )));
        }

        selected.push(SelectedCandidate {
            id: Some(candidate.id.clone()),
            path,
            bytes: candidate.bytes,
            rule_id: draft.rule_id,
            category: draft.category,
            safety: draft.safety,
            risk_score: candidate.risk_score,
        });
    }
    Ok(selected)
}

fn classify_plan_candidate(
    plan: &ActionPlan,
    candidate: &PlanCandidate,
    path: &Path,
) -> Option<CandidateDraft> {
    plan.projects
        .iter()
        .filter(|project| {
            project
                .candidates
                .iter()
                .any(|project_candidate| project_candidate.path == candidate.path)
        })
        .find_map(|project| classify_from_project_context(plan, project, path))
        .or_else(|| classify_from_path_parent(plan, path))
}

fn classify_from_project_context(
    plan: &ActionPlan,
    project: &ProjectReport,
    path: &Path,
) -> Option<CandidateDraft> {
    let project_dir = PathBuf::from(&project.path);
    let relative = path.strip_prefix(&project_dir).ok()?;
    let first_component = relative.components().next()?;
    let Component::Normal(name) = first_component else {
        return None;
    };
    let name = name.to_str()?;
    let classifier_path = project_dir.join(name);
    let draft = classify_from_project_rules(plan, &project_dir, name, classifier_path)?;
    (draft.path == path).then_some(draft)
}

fn classify_from_path_parent(plan: &ActionPlan, path: &Path) -> Option<CandidateDraft> {
    let parent = path.parent()?;
    let name = path.file_name()?.to_str()?;
    classify_from_project_rules(plan, parent, name, path.to_path_buf())
}

fn classify_from_project_rules(
    plan: &ActionPlan,
    project_dir: &Path,
    name: &str,
    path: PathBuf,
) -> Option<CandidateDraft> {
    rules::classify_candidate(project_dir, name, path)
        .or_else(|| classify_from_user_rules(plan, project_dir, name))
}

fn classify_from_user_rules(
    plan: &ActionPlan,
    project_dir: &Path,
    name: &str,
) -> Option<CandidateDraft> {
    let root = matching_plan_root(plan, project_dir)?;
    let user_rules = UserRuleSet::load_from_root(&root);
    if user_rules.is_empty() {
        return None;
    }
    user_rules.classify(name, project_dir)
}

fn matching_plan_root(plan: &ActionPlan, path: &Path) -> Option<PathBuf> {
    plan.roots
        .iter()
        .map(PathBuf::from)
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count())
}

pub fn revalidate_selected(
    plan: &ActionPlan,
    selected: &[SelectedCandidate],
) -> Result<(), PlanError> {
    let roots = plan
        .roots
        .iter()
        .filter_map(|root| PathBuf::from(root).canonicalize().ok())
        .collect::<Vec<_>>();
    if roots.is_empty() {
        return Err(PlanError::Generic(
            "action plan has no valid canonical roots".to_string(),
        ));
    }

    for candidate in selected {
        let metadata = fs::symlink_metadata(&candidate.path).map_err(|source| PlanError::Io {
            path: candidate.path.clone(),
            source,
        })?;
        if metadata.file_type().is_symlink() {
            return Err(PlanError::Generic(format!(
                "{} is now a symlink",
                candidate.path.display()
            )));
        }
        if !metadata.is_dir() {
            return Err(PlanError::Generic(format!(
                "{} is not a directory",
                candidate.path.display()
            )));
        }
        let canonical = candidate
            .path
            .canonicalize()
            .map_err(|source| PlanError::Io {
                path: candidate.path.clone(),
                source,
            })?;
        if !roots.iter().any(|root| canonical.starts_with(root)) {
            return Err(PlanError::Generic(format!(
                "{} resolves outside the action plan roots",
                candidate.path.display()
            )));
        }
    }

    Ok(())
}

/// Build a `Summary` that reflects what is actually in `selected`, while
/// preserving the scan-wide accounting (`projects_scanned`,
/// `projects_with_candidates`) from the original report. Without this,
/// `ActionPlan.summary` would describe the entire scan instead of the
/// chosen-for-deletion subset, misleading downstream consumers.
fn summarize_selected(selected: &[PlanCandidate], scan_summary: &Summary) -> Summary {
    let mut summary = Summary {
        projects_scanned: scan_summary.projects_scanned,
        projects_with_candidates: scan_summary.projects_with_candidates,
        ..Summary::default()
    };
    for candidate in selected {
        summary.candidates += 1;
        summary.total_bytes = summary.total_bytes.saturating_add(candidate.bytes);
        match candidate.safety {
            Safety::Safe => summary.safe_candidates += 1,
            Safety::Caution => summary.caution_candidates += 1,
            Safety::Blocked => summary.blocked_candidates += 1,
            Safety::Unknown => {}
        }
    }
    summary
}

fn collect_selected(report: &ScanReport, include_caution: bool) -> Vec<PlanCandidate> {
    report
        .projects
        .iter()
        .flat_map(|project| project.candidates.iter())
        .filter(|candidate| {
            candidate.safety == Safety::Safe
                || (include_caution && candidate.safety == Safety::Caution)
        })
        .map(to_plan_candidate)
        .collect()
}

#[cfg_attr(not(feature = "tui"), allow(dead_code))]
fn collect_selected_paths(
    report: &ScanReport,
    selected: &[SelectedCandidate],
) -> Vec<PlanCandidate> {
    let selected_paths = selected
        .iter()
        .map(|candidate| candidate.path.display().to_string())
        .collect::<std::collections::HashSet<_>>();

    report
        .projects
        .iter()
        .flat_map(|project| project.candidates.iter())
        .filter(|candidate| selected_paths.contains(&candidate.path))
        .map(to_plan_candidate)
        .collect()
}

fn to_plan_candidate(candidate: &Candidate) -> PlanCandidate {
    PlanCandidate {
        id: generate_candidate_id(),
        path: candidate.path.clone(),
        rule_id: candidate.rule_id.clone(),
        bytes: candidate.bytes,
        safety: candidate.safety,
        category: candidate.category,
        risk_score: candidate.risk_score,
    }
}

fn validate_delete_mode(delete_mode: &str) -> Result<(), PlanError> {
    match delete_mode {
        "trash" | "graveyard" | "permanent" => Ok(()),
        other => Err(PlanError::Generic(format!(
            "unsupported action plan deleteMode {other:?}; expected trash, graveyard, or permanent"
        ))),
    }
}

fn generate_candidate_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let timestamp_ms = now.as_millis() & ((1u128 << 48) - 1);
    let counter = PLAN_ID_COUNTER.fetch_add(1, Ordering::Relaxed) as u128;
    let entropy = ((std::process::id() as u128 & 0xffff) << 64)
        | ((counter & 0x0000_ffff_ffff_ffff) << 16)
        | (now.subsec_nanos() as u128 & 0xffff);
    encode_ulid((timestamp_ms << 80) | entropy)
}

fn encode_ulid(mut value: u128) -> String {
    const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut out = [b'0'; 26];
    for index in (0..out.len()).rev() {
        out[index] = CROCKFORD[(value & 0b1_1111) as usize];
        value >>= 5;
    }
    String::from_utf8(out.to_vec()).expect("ULID alphabet is valid UTF-8")
}

static PLAN_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::model::{ActivityInfo, Category, ProjectReport, ScanReport, Summary};

    fn report(root: &Path, candidate_path: &Path) -> ScanReport {
        ScanReport {
            schema_version: 1,
            tool_version: "0.1.0".to_string(),
            scanned_at: "2026-05-06T00:00:00Z".to_string(),
            roots: vec![root.display().to_string()],
            summary: Summary {
                projects_scanned: 1,
                projects_with_candidates: 1,
                candidates: 1,
                safe_candidates: 1,
                caution_candidates: 0,
                blocked_candidates: 0,
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
}
