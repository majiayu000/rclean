use std::env;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

#[cfg(any(target_os = "macos", test))]
use super::macos::{parse_defaults_bool, parse_defaults_string, parse_pmset_assertions};
use super::process::{parse_agent_processes, redact_home_path};
use super::system::run_output;
#[cfg(target_os = "macos")]
use super::system::run_status;
use super::types::{
    AgentError, AgentProcess, AgentReport, AgentTool, AutoUpdateStatus, DiskEntry, OptimizeAction,
    OptimizeOptions, OptimizeResult, PowerReport,
};

const CODEX_DEFAULTS_DOMAIN: &str = "com.openai.codex";

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
        path: redact_home_path(&path.display().to_string()),
        exists,
        bytes,
    }
}

fn path_size(path: &Path, warnings: &mut Vec<String>) -> u64 {
    if path.is_file() {
        return match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata.len(),
            Err(err) => {
                warnings.push(format!(
                    "cannot stat {}: {err}",
                    redact_home_path(&path.display().to_string())
                ));
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
                Err(err) => warnings.push(format!(
                    "cannot stat {}: {err}",
                    redact_home_path(&entry.path().display().to_string())
                )),
            },
            Err(err) => warnings.push(format!(
                "cannot read {}: {err}",
                redact_home_path(&path.display().to_string())
            )),
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
