use std::fs;
use std::io::Write as _;
use std::path::Path;

use chrono::Utc;
use serde_json::Value;

use crate::clean::SelectedCandidate;
use crate::error::PlanError;
use crate::model::ScanReport;

use super::schema::{ACTION_PLAN_SCHEMA_VERSION, ActionPlan};
use super::selection::{collect_selected, collect_selected_paths, summarize_selected};

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

pub fn write_selected_action_plan(
    report: &ScanReport,
    path: &Path,
    selected: &[SelectedCandidate],
    delete_mode: &str,
) -> Result<(), PlanError> {
    let selected = collect_selected_paths(report, selected);
    let summary = summarize_selected(&selected, &report.summary);
    let plan = ActionPlan {
        schema_version: ACTION_PLAN_SCHEMA_VERSION,
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

fn validate_delete_mode(delete_mode: &str) -> Result<(), PlanError> {
    match delete_mode {
        "trash" | "graveyard" | "permanent" => Ok(()),
        other => Err(PlanError::Generic(format!(
            "unsupported action plan deleteMode {other:?}; expected trash, graveyard, or permanent"
        ))),
    }
}
