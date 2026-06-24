use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::clean::SelectedCandidate;
use crate::error::PlanError;
use crate::model::{CandidateDraft, ProjectReport, Safety};
use crate::path_util::path_file_name;
use crate::rules;
use crate::scan::{dangerous_link_kind, is_protected_user_data_path, is_runtime_or_system_path};
use crate::user_rules::UserRuleSet;

use super::schema::{ActionPlan, PlanCandidate};

pub fn selected_from_action_plan(plan: &ActionPlan) -> Result<Vec<SelectedCandidate>, PlanError> {
    let mut selected = Vec::with_capacity(plan.selected.len());
    for candidate in &plan.selected {
        let path = PathBuf::from(&candidate.path);
        if candidate.requires_sudo {
            return Err(PlanError::Generic(requires_sudo_plan_reason(
                &candidate.path,
                &candidate.rule_id,
            )));
        }
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

        if rules::requires_sudo(&draft.rule_id) {
            return Err(PlanError::Generic(requires_sudo_plan_reason(
                &candidate.path,
                &draft.rule_id,
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
            requires_sudo: rules::requires_sudo(&candidate.rule_id),
            risk_score: candidate.risk_score,
        });
    }
    Ok(selected)
}

fn requires_sudo_plan_reason(path: &str, rule_id: &str) -> String {
    format!(
        "{path} requires administrator access for rule {rule_id}; rclean will not run sudo or delete it. Review and remove it manually with administrator privileges if desired"
    )
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
        if let Some(kind) = dangerous_link_kind(&metadata) {
            return Err(PlanError::Generic(format!(
                "{} is now a {}",
                candidate.path.display(),
                kind.description()
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
    if let Some(draft) = classify_agent_tmp_worktree_from_plan(plan, candidate, path) {
        return Some(draft);
    }

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

fn classify_agent_tmp_worktree_from_plan(
    plan: &ActionPlan,
    candidate: &PlanCandidate,
    path: &Path,
) -> Option<CandidateDraft> {
    if candidate.rule_id != "agent.tmp_worktree" || !is_immediate_child_of_tmp_plan_root(plan, path)
    {
        return None;
    }

    let parent = path.parent()?;
    let name = path_file_name(path)?;
    rules::classify_agent_tmp_worktree(parent, name, path)
}

fn is_immediate_child_of_tmp_plan_root(plan: &ActionPlan, path: &Path) -> bool {
    plan.roots.iter().map(PathBuf::from).any(|root| {
        is_allowed_tmp_plan_root(&root)
            && path.strip_prefix(&root).is_ok_and(|relative| {
                let mut components = relative.components();
                matches!(components.next(), Some(Component::Normal(_)))
                    && components.next().is_none()
            })
    })
}

fn is_allowed_tmp_plan_root(root: &Path) -> bool {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    candidate_tmp_roots()
        .into_iter()
        .filter_map(|tmp_root| tmp_root.canonicalize().ok())
        .any(|tmp_root| tmp_root == root)
}

fn candidate_tmp_roots() -> Vec<PathBuf> {
    if let Some(roots) = std::env::var_os("RCLEAN_TMP_ROOTS") {
        return std::env::split_paths(&roots).collect();
    }

    default_tmp_roots()
}

#[cfg(target_os = "macos")]
fn default_tmp_roots() -> Vec<PathBuf> {
    let mut roots = vec![std::env::temp_dir()];
    roots.push(PathBuf::from("/private/tmp"));
    roots.push(PathBuf::from("/tmp"));
    roots
}

#[cfg(target_os = "windows")]
fn default_tmp_roots() -> Vec<PathBuf> {
    vec![std::env::temp_dir()]
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn default_tmp_roots() -> Vec<PathBuf> {
    let mut roots = vec![std::env::temp_dir()];
    roots.push(PathBuf::from("/tmp"));
    roots
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
