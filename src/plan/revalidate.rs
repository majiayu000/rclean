use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::clean::SelectedCandidate;
use crate::error::PlanError;
use crate::model::{CandidateDraft, ProjectReport, Safety};
use crate::path_util::path_file_name;
use crate::rules;
use crate::scan::{is_protected_user_data_path, is_runtime_or_system_path};
use crate::user_rules::UserRuleSet;

use super::schema::{ActionPlan, PlanCandidate};

pub fn selected_from_action_plan(plan: &ActionPlan) -> Result<Vec<SelectedCandidate>, PlanError> {
    let mut selected = Vec::with_capacity(plan.selected.len());
    for candidate in &plan.selected {
        let path = PathBuf::from(&candidate.path);
        if is_protected_user_data_path(&path)
            && !rules::allows_protected_user_data_path(&candidate.rule_id)
        {
            return Err(PlanError::Generic(format!(
                "{} is protected user data; refusing to clean",
                candidate.path
            )));
        }

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

        if draft.safety == Safety::Blocked
            || draft.safety == Safety::Unknown
            || draft.safety == Safety::ReportOnly
        {
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
        if is_protected_user_data_path(&canonical)
            && !rules::allows_protected_user_data_path(&candidate.rule_id)
        {
            return Err(PlanError::Generic(format!(
                "{} resolves to protected user data",
                candidate.path.display()
            )));
        }
        if !roots.iter().any(|root| canonical.starts_with(root)) {
            return Err(PlanError::Generic(format!(
                "{} resolves outside the action plan roots",
                candidate.path.display()
            )));
        }
    }

    Ok(())
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
    let name = path_file_name(path)?;
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
