//! Diagnostics and one-shot optimization helpers for local AI agent tools.
//!
//! This module is deliberately separate from `scan` and `clean`: agent
//! settings are app/system state, not rebuildable developer artifacts.

use std::env;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

const CODEX_DEFAULTS_DOMAIN: &str = "com.openai.codex";

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

pub fn diagnose_agent(tool: AgentTool) -> AgentReport {
    match tool {
        AgentTool::Codex => diagnose_codex(),
    }
}

pub fn optimize(options: OptimizeOptions) -> Result<OptimizeResult, AgentError> {
    if !options.disable_auto_update {
        return Err(AgentError::NoOptimizationSelected);
    }

    match options.tool {
        AgentTool::Codex => optimize_codex(options),
    }
}

fn diagnose_codex() -> AgentReport {
    let mut warnings = Vec::new();
    let processes = read_processes(AgentTool::Codex, &mut warnings);
    let power = read_power(AgentTool::Codex, &mut warnings);
    let disk = codex_disk_entries(&mut warnings);
    let auto_update = read_codex_auto_update(&mut warnings);

    let mut report = AgentReport {
        schema_version: 1,
        tool: AgentTool::Codex,
        generated_at: chrono::Utc::now().to_rfc3339(),
        processes,
        power,
        disk,
        auto_update,
        warnings,
        recommendations: Vec::new(),
    };
    report.recommendations = codex_recommendations(&report);
    report
}

fn optimize_codex(options: OptimizeOptions) -> Result<OptimizeResult, AgentError> {
    let defaults_domain = codex_defaults_domain(options.codex_defaults_domain.as_deref());
    let action = OptimizeAction {
        id: "codex.disable_auto_update".to_string(),
        description: "Disable Sparkle automatic update checks for Codex.app".to_string(),
        commands: vec![
            format!("defaults write {defaults_domain} SUAutomaticallyUpdate -bool false"),
            format!("defaults write {defaults_domain} SUEnableAutomaticChecks -bool false"),
        ],
        status: if options.apply {
            "applied".to_string()
        } else {
            "dry-run".to_string()
        },
    };

    if options.apply {
        apply_codex_disable_auto_update(defaults_domain)?;
    }

    Ok(OptimizeResult {
        tool: AgentTool::Codex,
        applied: options.apply,
        actions: vec![action],
    })
}

#[cfg(target_os = "macos")]
fn apply_codex_disable_auto_update(defaults_domain: &str) -> Result<(), AgentError> {
    run_status(
        "defaults",
        &[
            "write",
            defaults_domain,
            "SUAutomaticallyUpdate",
            "-bool",
            "false",
        ],
    )?;
    run_status(
        "defaults",
        &[
            "write",
            defaults_domain,
            "SUEnableAutomaticChecks",
            "-bool",
            "false",
        ],
    )?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn apply_codex_disable_auto_update(_defaults_domain: &str) -> Result<(), AgentError> {
    Err(AgentError::UnsupportedApplyPlatform {
        tool: AgentTool::Codex,
    })
}

fn codex_defaults_domain(override_domain: Option<&str>) -> &str {
    override_domain
        .filter(|domain| !domain.trim().is_empty())
        .unwrap_or(CODEX_DEFAULTS_DOMAIN)
}

fn read_processes(tool: AgentTool, warnings: &mut Vec<String>) -> Vec<AgentProcess> {
    match run_output("ps", &["-axo", "pid=,command="]) {
        Ok(output) => parse_agent_processes(tool, &output),
        Err(err) => {
            warnings.push(err.to_string());
            Vec::new()
        }
    }
}

fn read_power(tool: AgentTool, warnings: &mut Vec<String>) -> PowerReport {
    #[cfg(target_os = "macos")]
    {
        match run_output("pmset", &["-g", "assertions"]) {
            Ok(output) => parse_pmset_assertions(tool, &output),
            Err(err) => {
                warnings.push(err.to_string());
                PowerReport {
                    supported: true,
                    ..PowerReport::default()
                }
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = tool;
        let _ = warnings;
        PowerReport::default()
    }
}

fn read_codex_auto_update(warnings: &mut Vec<String>) -> AutoUpdateStatus {
    #[cfg(target_os = "macos")]
    {
        match run_output("defaults", &["read", "com.openai.codex"]) {
            Ok(output) => AutoUpdateStatus {
                supported: true,
                automatically_update: parse_defaults_bool(&output, "SUAutomaticallyUpdate"),
                automatic_checks: parse_defaults_bool(&output, "SUEnableAutomaticChecks"),
                last_check_time: parse_defaults_string(&output, "SULastCheckTime"),
            },
            Err(err) => {
                warnings.push(err.to_string());
                AutoUpdateStatus {
                    supported: true,
                    ..AutoUpdateStatus::default()
                }
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = warnings;
        AutoUpdateStatus::default()
    }
}

fn codex_disk_entries(warnings: &mut Vec<String>) -> Vec<DiskEntry> {
    let mut paths = Vec::new();
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        paths.push(("codex.sessions", home.join(".codex").join("sessions")));
        paths.push((
            "codex.chronicle_memories",
            home.join(".codex")
                .join("memories")
                .join("extensions")
                .join("chronicle"),
        ));
        paths.push((
            "codex.application_support",
            home.join("Library")
                .join("Application Support")
                .join("Codex"),
        ));
    } else {
        warnings.push("HOME is not set; skipped home-based Codex paths".to_string());
    }

    let temp = env::temp_dir();
    paths.push(("codex.chronicle_tmp", temp.join("chronicle")));
    paths.push(("codex.chronicle_tmp_legacy", temp.join("codex_chronicle")));

    paths
        .into_iter()
        .map(|(label, path)| disk_entry(label, path, warnings))
        .collect()
}

fn disk_entry(label: &str, path: PathBuf, warnings: &mut Vec<String>) -> DiskEntry {
    let exists = path.exists();
    let bytes = if exists {
        path_size(&path, warnings)
    } else {
        0
    };

    DiskEntry {
        label: label.to_string(),
        path: path.display().to_string(),
        exists,
        bytes,
    }
}

fn path_size(path: &Path, warnings: &mut Vec<String>) -> u64 {
    if path.is_file() {
        return match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata.len(),
            Err(err) => {
                warnings.push(format!("cannot stat {}: {err}", path.display()));
                0
            }
        };
    }

    let mut total = 0;
    for entry in WalkDir::new(path).follow_links(false) {
        match entry {
            Ok(entry) => match std::fs::symlink_metadata(entry.path()) {
                Ok(metadata) if metadata.is_file() => total += metadata.len(),
                Ok(_) => {}
                Err(err) => warnings.push(format!("cannot stat {}: {err}", entry.path().display())),
            },
            Err(err) => warnings.push(format!("cannot read {}: {err}", path.display())),
        }
    }
    total
}

fn codex_recommendations(report: &AgentReport) -> Vec<String> {
    let mut recommendations = Vec::new();

    if report.power.agent_blocks_display_sleep {
        recommendations.push(
            "Codex is holding a display-sleep assertion; disable Chronicle/capturing or quit Codex when idle"
                .to_string(),
        );
    }
    if report.auto_update.automatically_update == Some(true)
        || report.auto_update.automatic_checks == Some(true)
    {
        recommendations.push(
            "Run `rclean agent optimize codex --disable-auto-update --yes` to disable Codex Sparkle auto updates"
                .to_string(),
        );
    }
    if report
        .disk
        .iter()
        .any(|entry| entry.bytes >= 1024 * 1024 * 1024)
    {
        recommendations.push(
            "Large local Codex state detected; review sessions/Chronicle paths before deleting anything"
                .to_string(),
        );
    }
    recommendations.push(
        "Local diagnostics cannot prove account token usage; they only report local process, disk, and power signals"
            .to_string(),
    );

    recommendations
}

fn parse_agent_processes(tool: AgentTool, output: &str) -> Vec<AgentProcess> {
    output
        .lines()
        .filter_map(parse_process_line)
        .filter(|process| process_matches_tool(tool, &process.command))
        .map(|process| AgentProcess {
            pid: process.pid,
            command: compact_command(&process.command),
        })
        .collect()
}

fn parse_process_line(line: &str) -> Option<AgentProcess> {
    let trimmed = line.trim();
    let split_at = trimmed.find(char::is_whitespace)?;
    let (pid, command) = trimmed.split_at(split_at);
    Some(AgentProcess {
        pid: pid.parse().ok()?,
        command: command.trim().to_string(),
    })
}

fn process_matches_tool(tool: AgentTool, command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    match tool {
        AgentTool::Codex => {
            lower == "codex"
                || lower.starts_with("/applications/codex.app/")
                || lower.contains("/codex.app/")
                || lower.contains("codex computer use.app/")
                || lower.contains("skycomputeruseclient")
                || lower.contains("/node_modules/@openai/codex/")
                || lower.contains("/bin/codex ")
                || lower.ends_with("/bin/codex")
                || lower.contains("/resources/codex ")
                || lower.contains(" codex app-server")
                || lower.contains("codex_chronicle")
        }
    }
}

fn compact_command(command: &str) -> String {
    const MAX_CHARS: usize = 240;
    let mut chars = command.chars();
    let prefix: String = chars.by_ref().take(MAX_CHARS).collect();
    if chars.next().is_some() {
        format!("{prefix}...")
    } else {
        prefix
    }
}

#[cfg(any(target_os = "macos", test))]
fn parse_pmset_assertions(tool: AgentTool, output: &str) -> PowerReport {
    let all_assertions: Vec<PowerAssertion> = output
        .lines()
        .filter_map(parse_pmset_assertion_line)
        .collect();
    let assertions: Vec<PowerAssertion> = all_assertions
        .into_iter()
        .filter(|assertion| process_matches_tool(tool, &assertion.process))
        .collect();
    let agent_blocks_display_sleep = assertions
        .iter()
        .any(|assertion| assertion.kind.contains("NoDisplaySleep"));

    PowerReport {
        supported: true,
        prevent_user_idle_display_sleep: parse_pmset_summary_bool(
            output,
            "PreventUserIdleDisplaySleep",
        ),
        prevent_user_idle_system_sleep: parse_pmset_summary_bool(
            output,
            "PreventUserIdleSystemSleep",
        ),
        agent_blocks_display_sleep,
        assertions,
    }
}

#[cfg(any(target_os = "macos", test))]
fn parse_pmset_summary_bool(output: &str, key: &str) -> Option<bool> {
    output.lines().find_map(|line| {
        let trimmed = line.trim();
        if !trimmed.starts_with(key) {
            return None;
        }
        trimmed.split_whitespace().last().map(|value| value == "1")
    })
}

#[cfg(any(target_os = "macos", test))]
fn parse_pmset_assertion_line(line: &str) -> Option<PowerAssertion> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("pid ")?;
    let (pid_text, after_pid) = rest.split_once('(')?;
    let (process, after_process) = after_pid.split_once("):")?;
    let pid = pid_text.parse().ok()?;
    let after_bracket = after_process
        .split_once(']')
        .map(|(_, suffix)| suffix.trim())
        .unwrap_or(after_process.trim());
    let mut parts = after_bracket.split_whitespace();
    let age = parts.next().map(str::to_string);
    let kind = parts.next().unwrap_or("").to_string();
    let name = after_bracket
        .split_once("named:")
        .map(|(_, name)| name.trim().trim_matches('"').to_string())
        .filter(|name| !name.is_empty());

    Some(PowerAssertion {
        pid,
        process: process.to_string(),
        age,
        kind,
        name,
    })
}

#[cfg(any(target_os = "macos", test))]
fn parse_defaults_bool(output: &str, key: &str) -> Option<bool> {
    output.lines().find_map(|line| {
        let trimmed = line.trim();
        if !trimmed.starts_with(key) {
            return None;
        }
        let (_, value) = trimmed.split_once('=')?;
        match value.trim().trim_end_matches(';') {
            "1" | "true" | "TRUE" | "YES" => Some(true),
            "0" | "false" | "FALSE" | "NO" => Some(false),
            _ => None,
        }
    })
}

#[cfg(any(target_os = "macos", test))]
fn parse_defaults_string(output: &str, key: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let trimmed = line.trim();
        if !trimmed.starts_with(key) {
            return None;
        }
        let (_, value) = trimmed.split_once('=')?;
        Some(
            value
                .trim()
                .trim_end_matches(';')
                .trim_matches('"')
                .to_string(),
        )
        .filter(|value| !value.is_empty())
    })
}

fn run_output(program: &str, args: &[&str]) -> Result<String, AgentError> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|source| AgentError::CommandIo {
            program: program.to_string(),
            source,
        })?;
    if !output.status.success() {
        return Err(AgentError::CommandFailed {
            program: program.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(target_os = "macos")]
fn run_status(program: &str, args: &[&str]) -> Result<(), AgentError> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|source| AgentError::CommandIo {
            program: program.to_string(),
            source,
        })?;
    if !output.status.success() {
        return Err(AgentError::CommandFailed {
            program: program.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_codex_display_sleep_assertion() {
        let output = r#"
   PreventUserIdleDisplaySleep    1
   PreventUserIdleSystemSleep     1
   pid 9108(Codex): [0x000198f10005985f] 14:58:28 NoDisplaySleepAssertion named: "Capturing"
"#;

        let report = parse_pmset_assertions(AgentTool::Codex, output);

        assert_eq!(report.prevent_user_idle_display_sleep, Some(true));
        assert!(report.agent_blocks_display_sleep);
        assert_eq!(report.assertions.len(), 1);
        assert_eq!(report.assertions[0].name.as_deref(), Some("Capturing"));
    }

    #[test]
    fn parses_codex_defaults_update_keys() {
        let output = r#"
{
    SUAutomaticallyUpdate = 1;
    SUEnableAutomaticChecks = 0;
    SULastCheckTime = "2026-05-21 03:35:51 +0000";
}
"#;

        assert_eq!(
            parse_defaults_bool(output, "SUAutomaticallyUpdate"),
            Some(true)
        );
        assert_eq!(
            parse_defaults_bool(output, "SUEnableAutomaticChecks"),
            Some(false)
        );
        assert_eq!(
            parse_defaults_string(output, "SULastCheckTime").as_deref(),
            Some("2026-05-21 03:35:51 +0000")
        );
    }

    #[test]
    fn optimize_dry_run_uses_override_defaults_domain() {
        let result = optimize(OptimizeOptions {
            tool: AgentTool::Codex,
            disable_auto_update: true,
            apply: false,
            codex_defaults_domain: Some("com.openai.rclean-sandbox".to_string()),
        })
        .unwrap();

        assert_eq!(
            result.actions[0].commands[0],
            "defaults write com.openai.rclean-sandbox SUAutomaticallyUpdate -bool false"
        );
    }

    #[test]
    fn filters_codex_related_processes() {
        let output = r#"
  100 /Applications/Codex.app/Contents/MacOS/Codex
  101 /usr/bin/ssh-agent
  102 /Applications/Codex.app/Contents/Resources/app.asar.unpacked/node_modules/@openai/screen-capture-child
  103 target/debug/rclean agent doctor codex
  104 node /Users/me/.local/share/fnm/node-versions/v25/bin/codex --dangerously-bypass-approvals-and-sandbox
"#;

        let processes = parse_agent_processes(AgentTool::Codex, output);

        assert_eq!(processes.len(), 3);
        assert_eq!(processes[0].pid, 100);
        assert_eq!(processes[1].pid, 102);
        assert_eq!(processes[2].pid, 104);
    }
}
