use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::clean::SelectedCandidate;
use crate::model::{Candidate, ProjectReport, Safety, ScanReport, Summary};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
    fs::write(path, json).map_err(|err| format!("failed to write {}: {err}", path.display()))
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
    let selected = plan
        .selected
        .iter()
        .filter(|candidate| candidate.safety != Safety::Blocked)
        .map(|candidate| SelectedCandidate {
            path: PathBuf::from(&candidate.path),
            bytes: candidate.bytes,
            rule_id: candidate.rule_id.clone(),
        })
        .collect::<Vec<_>>();
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
            }],
        }
    }

    #[test]
    fn writes_and_revalidates_plan() {
        let temp = TempDir::new().unwrap();
        let candidate = temp.path().join("node_modules");
        fs::create_dir(&candidate).unwrap();
        let plan_path = temp.path().join("plan.json");
        let report = report(temp.path(), &candidate);

        write_action_plan(&report, &plan_path, false, false, "trash").unwrap();
        let plan = read_action_plan(&plan_path).unwrap();
        let selected = selected_from_action_plan(&plan).unwrap();

        assert_eq!(selected.len(), 1);
        revalidate_selected(&plan, &selected).unwrap();
    }
}
