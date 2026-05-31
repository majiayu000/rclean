use serde::{Deserialize, Serialize};

use crate::model::{Category, ProjectReport, Safety, Summary};

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
