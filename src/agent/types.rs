use std::fmt;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentTool {
    Codex,
}

impl fmt::Display for AgentTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            AgentTool::Codex => "codex",
        })
    }
}

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("select at least one agent optimization flag")]
    NoOptimizationSelected,
    #[cfg(not(target_os = "macos"))]
    #[error("{tool} optimization can only be applied on macOS")]
    UnsupportedApplyPlatform { tool: AgentTool },
    #[error("failed to run {program}: {source}")]
    CommandIo {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("command failed: {program} {args:?}: {stderr}")]
    CommandFailed {
        program: String,
        args: Vec<String>,
        stderr: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentReport {
    pub schema_version: u32,
    pub tool: AgentTool,
    pub generated_at: String,
    pub processes: Vec<AgentProcess>,
    pub power: PowerReport,
    pub disk: Vec<DiskEntry>,
    pub auto_update: AutoUpdateStatus,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProcess {
    pub pid: u32,
    pub command: String,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerReport {
    pub supported: bool,
    pub prevent_user_idle_display_sleep: Option<bool>,
    pub prevent_user_idle_system_sleep: Option<bool>,
    pub agent_blocks_display_sleep: bool,
    pub assertions: Vec<PowerAssertion>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerAssertion {
    pub pid: u32,
    pub process: String,
    pub age: Option<String>,
    pub kind: String,
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskEntry {
    pub label: String,
    pub path: String,
    pub exists: bool,
    pub bytes: u64,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateStatus {
    pub supported: bool,
    pub automatically_update: Option<bool>,
    pub automatic_checks: Option<bool>,
    pub last_check_time: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OptimizeOptions {
    pub tool: AgentTool,
    pub disable_auto_update: bool,
    pub apply: bool,
    pub codex_defaults_domain: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeResult {
    pub tool: AgentTool,
    pub applied: bool,
    pub actions: Vec<OptimizeAction>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeAction {
    pub id: String,
    pub description: String,
    pub commands: Vec<String>,
    pub status: String,
}
