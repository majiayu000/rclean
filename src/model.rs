use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Deps,
    Build,
    Cache,
    Test,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Category::Deps => "deps",
            Category::Build => "build",
            Category::Cache => "cache",
            Category::Test => "test",
        })
    }
}

impl FromStr for Category {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "deps" => Ok(Category::Deps),
            "build" => Ok(Category::Build),
            "cache" => Ok(Category::Cache),
            "test" => Ok(Category::Test),
            other => Err(format!(
                "invalid category '{other}'. Use deps, build, cache, or test"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Safety {
    Safe,
    Caution,
    Blocked,
    /// User data, not a rebuildable cache. Reported for awareness but
    /// never selected for cleanup — even with `--include-blocked`.
    /// Used for paths like `~/.ollama/models` where re-acquiring the
    /// content costs hours of network time and the user must
    /// explicitly opt in per path.
    #[serde(rename = "report-only")]
    ReportOnly,
    Unknown,
}

impl fmt::Display for Safety {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Safety::Safe => "safe",
            Safety::Caution => "caution",
            Safety::Blocked => "blocked",
            Safety::ReportOnly => "report-only",
            Safety::Unknown => "unknown",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub schema_version: u32,
    pub tool_version: String,
    pub scanned_at: String,
    pub roots: Vec<String>,
    pub summary: Summary,
    pub projects: Vec<ProjectReport>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub projects_scanned: usize,
    pub projects_with_candidates: usize,
    pub candidates: usize,
    pub safe_candidates: usize,
    pub caution_candidates: usize,
    pub blocked_candidates: usize,
    #[serde(default)]
    pub report_only_candidates: usize,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectReport {
    pub path: String,
    pub kind: String,
    pub markers: Vec<String>,
    pub git: Option<GitInfo>,
    pub activity: ActivityInfo,
    pub candidates: Vec<Candidate>,
    pub total_bytes: u64,
    pub project_bytes: u64,
    pub artifact_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub repo_root: String,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityInfo {
    pub last_modified: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub path: String,
    pub name: String,
    pub rule_id: String,
    pub category: Category,
    pub bytes: u64,
    pub safety: Safety,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub restore_hint: String,
    /// Composite risk signal in `[0.0, 1.0]` (final). Today the
    /// implementation reaches at most **0.85** because the root_boundary
    /// axis (weight 0.15) is deferred — see `compute_risk_score` for
    /// the full formula and rationale.
    ///
    /// Independent of the safe/caution/blocked tier: the safety tier
    /// still controls auto-selection; risk_score is an advisory signal
    /// for downstream consumers (TUI coloring, AI agents scoring a
    /// plan, etc).
    #[serde(default)]
    pub risk_score: f32,
}

#[derive(Debug, Clone)]
pub struct CandidateDraft {
    pub path: PathBuf,
    pub name: String,
    pub rule_id: String,
    pub category: Category,
    pub safety: Safety,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub restore_hint: String,
}

#[derive(Debug, Clone)]
pub struct Explanation {
    pub path: PathBuf,
    pub safety: Safety,
    pub rule_id: Option<String>,
    pub category: Option<Category>,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub restore_hint: Option<String>,
    /// Composite risk signal for the candidate, computed against its
    /// project directory. `None` when no built-in rule matched (the
    /// path is `Safety::Unknown` and risk is undefined). See
    /// `Candidate.risk_score` for the formula and current 0.85 cap.
    pub risk_score: Option<f32>,
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else if value >= 10.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
