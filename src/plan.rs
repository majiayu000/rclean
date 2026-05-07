use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::clean::SelectedCandidate;
use crate::model::{Candidate, ProjectReport, Safety, ScanReport, Summary};
use crate::rules;
use crate::scan::is_runtime_or_system_path;

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
    pub path: String,
    pub rule_id: String,
    pub bytes: u64,
    pub safety: Safety,
}

pub fn write_action_plan(
    report: &ScanReport,
    path: &Path,
    include_caution: bool,
    include_permanent: bool,
    delete_mode: &str,
) -> Result<(), String> {
    let selected = collect_selected(report, include_caution);
    let plan = ActionPlan {
        schema_version: 1,
        tool_version: report.tool_version.clone(),
        generated_at: Utc::now().to_rfc3339(),
        delete_mode: if include_permanent {
            "permanent".to_string()
        } else {
            delete_mode.to_string()
        },
        roots: report.roots.clone(),
        summary: report.summary.clone(),
        selected,
        projects: report.projects.clone(),
    };
    let json = serde_json::to_string_pretty(&plan)
        .map_err(|err| format!("failed to serialize action plan: {err}"))?;
    write_atomically(path, json.as_bytes())
}

fn write_atomically(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    let mut tmp = match parent {
        Some(dir) => tempfile::NamedTempFile::new_in(dir),
        None => tempfile::NamedTempFile::new_in("."),
    }
    .map_err(|err| format!("failed to create temp file near {}: {err}", path.display()))?;

    use std::io::Write as _;
    tmp.write_all(contents)
        .map_err(|err| format!("failed to write temp file: {err}"))?;
    tmp.as_file()
        .sync_all()
        .map_err(|err| format!("failed to fsync temp file: {err}"))?;
    tmp.persist(path)
        .map_err(|err| format!("failed to rename into {}: {}", path.display(), err.error))?;
    Ok(())
}

pub fn read_action_plan(path: &Path) -> Result<ActionPlan, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let plan: ActionPlan =
        serde_json::from_str(&raw).map_err(|err| format!("invalid action plan: {err}"))?;
    if plan.schema_version != 1 {
        return Err(format!(
            "unsupported action plan schema version {}",
            plan.schema_version
        ));
    }
    Ok(plan)
}

pub fn selected_from_action_plan(plan: &ActionPlan) -> Result<Vec<SelectedCandidate>, String> {
    let mut selected = Vec::with_capacity(plan.selected.len());
    for candidate in &plan.selected {
        let path = PathBuf::from(&candidate.path);

        if is_runtime_or_system_path(&path) {
            return Err(format!(
                "{} is inside a protected runtime or system path; refusing to clean",
                candidate.path
            ));
        }

        let parent = path
            .parent()
            .ok_or_else(|| format!("plan candidate has no parent directory: {}", candidate.path))?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("plan candidate has invalid file name: {}", candidate.path))?;

        let draft = rules::classify_candidate(parent, name, path.clone()).ok_or_else(|| {
            format!(
                "{} is not recognized by any current rule (plan may be stale or tampered)",
                candidate.path
            )
        })?;

        if draft.safety == Safety::Blocked || draft.safety == Safety::Unknown {
            return Err(format!(
                "{} is now classified as {:?} by rule {}; refusing to clean",
                candidate.path, draft.safety, draft.rule_id
            ));
        }

        selected.push(SelectedCandidate {
            path,
            bytes: candidate.bytes,
            rule_id: draft.rule_id,
        });
    }
    Ok(selected)
}

pub fn revalidate_selected(
    plan: &ActionPlan,
    selected: &[SelectedCandidate],
) -> Result<(), String> {
    let roots = plan
        .roots
        .iter()
        .filter_map(|root| PathBuf::from(root).canonicalize().ok())
        .collect::<Vec<_>>();
    if roots.is_empty() {
        return Err("action plan has no valid canonical roots".to_string());
    }

    for candidate in selected {
        let metadata = fs::symlink_metadata(&candidate.path).map_err(|err| {
            format!(
                "{} no longer exists or cannot be read: {err}",
                candidate.path.display()
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Err(format!("{} is now a symlink", candidate.path.display()));
        }
        if !metadata.is_dir() {
            return Err(format!("{} is not a directory", candidate.path.display()));
        }
        let canonical = candidate
            .path
            .canonicalize()
            .map_err(|err| format!("failed to canonicalize {}: {err}", candidate.path.display()))?;
        if !roots.iter().any(|root| canonical.starts_with(root)) {
            return Err(format!(
                "{} resolves outside the action plan roots",
                candidate.path.display()
            ));
        }
    }

    Ok(())
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

fn to_plan_candidate(candidate: &Candidate) -> PlanCandidate {
    PlanCandidate {
        path: candidate.path.clone(),
        rule_id: candidate.rule_id.clone(),
        bytes: candidate.bytes,
        safety: candidate.safety,
    }
}

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
        let err = selected_from_action_plan(&plan).expect_err("should reject unrecognized paths");
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
            .expect_err("should reject a tampered plan that injects a system path");
        assert!(
            err.contains("/usr/local/lib/node_modules") || err.contains("not recognized"),
            "unexpected error: {err}"
        );
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
                "projects": [],
                "extraField": "should reject"
            }"#,
        )
        .unwrap();

        let err = read_action_plan(&plan_path).expect_err("should reject unknown fields");
        assert!(err.contains("unknown field"), "unexpected error: {err}");
    }
}
