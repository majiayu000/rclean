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
    Ide,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Category::Deps => "deps",
            Category::Build => "build",
            Category::Cache => "cache",
            Category::Test => "test",
            Category::Ide => "ide",
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
            "ide" => Ok(Category::Ide),
            other => Err(format!(
                "invalid category '{other}'. Use deps, build, cache, test, or ide"
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
    Unknown,
}

impl fmt::Display for Safety {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Safety::Safe => "safe",
            Safety::Caution => "caution",
            Safety::Blocked => "blocked",
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
