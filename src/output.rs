use crate::agent::{AgentReport, OptimizeResult};
use crate::doctor::{DoctorReport, Status};
use crate::model::{Candidate, Explanation, ProjectReport, Safety, ScanReport, format_bytes};
use crate::rules;

pub fn print_json(report: &ScanReport) -> Result<(), serde_json::Error> {
    let json = serde_json::to_string_pretty(report)?;
    println!("{json}");
    Ok(())
}

pub fn print_agent_report(report: &AgentReport) {
    println!("Agent: {}", report.tool);
    println!("Generated: {}", report.generated_at);

    println!();
    println!("Power:");
    if report.power.supported {
        println!(
            "  display sleep assertion: {}",
            format_bool(report.power.prevent_user_idle_display_sleep)
        );
        println!(
            "  idle system sleep assertion: {}",
            format_bool(report.power.prevent_user_idle_system_sleep)
        );
        println!(
            "  agent blocks display sleep: {}",
            if report.power.agent_blocks_display_sleep {
                "yes"
            } else {
                "no"
            }
        );
        for assertion in &report.power.assertions {
            println!(
                "  - pid {} {} {}{}",
                assertion.pid,
                assertion.process,
                assertion.kind,
                assertion
                    .name
                    .as_ref()
                    .map(|name| format!(" ({name})"))
                    .unwrap_or_default()
            );
        }
    } else {
        println!("  unsupported on this platform");
    }

    println!();
    println!("Disk:");
    for entry in &report.disk {
        let size = if entry.exists {
            format_bytes(entry.bytes)
        } else {
            "missing".to_string()
        };
        println!("  {:<28} {:>8}  {}", entry.label, size, entry.path);
    }

    println!();
    println!("Processes:");
    if report.processes.is_empty() {
        println!("  none detected");
    } else {
        let visible_processes = 20;
        for process in report.processes.iter().take(visible_processes) {
            println!("  - pid {} {}", process.pid, process.command);
        }
        if report.processes.len() > visible_processes {
            println!(
                "  ... and {} more process(es)",
                report.processes.len() - visible_processes
            );
        }
    }

    println!();
    println!("Auto update:");
    if report.auto_update.supported {
        println!(
            "  automatically update: {}",
            format_bool(report.auto_update.automatically_update)
        );
        println!(
            "  automatic checks: {}",
            format_bool(report.auto_update.automatic_checks)
        );
        if let Some(last_check) = &report.auto_update.last_check_time {
            println!("  last check: {last_check}");
        }
    } else {
        println!("  unsupported on this platform");
    }

    if !report.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &report.warnings {
            println!("  - {warning}");
        }
    }

    if !report.recommendations.is_empty() {
        println!();
        println!("Recommendations:");
        for recommendation in &report.recommendations {
            println!("  - {recommendation}");
        }
    }
}

pub fn print_agent_optimize_result(result: &OptimizeResult) {
    println!("Agent: {}", result.tool);
    println!(
        "Mode: {}",
        if result.applied { "applied" } else { "dry-run" }
    );
    for action in &result.actions {
        println!();
        println!("{}: {}", action.id, action.status);
        println!("  {}", action.description);
        for command in &action.commands {
            println!("  $ {command}");
        }
    }
    if !result.applied {
        println!();
        println!("Re-run with --yes to apply the selected changes.");
    }
}

fn format_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "on",
        Some(false) => "off",
        None => "unknown",
    }
}

pub fn print_table(report: &ScanReport) {
    println!();
    println!(
        "Summary: {} projects scanned, {} candidates, {} reclaimable",
        report.summary.projects_scanned,
        report.summary.candidates,
        format_bytes(report.summary.total_bytes)
    );
    println!(
        "Safety: {} safe, {} caution, {} blocked",
        report.summary.safe_candidates,
        report.summary.caution_candidates,
        report.summary.blocked_candidates
    );

    if report.projects.is_empty() {
        println!("No cleanable developer artifacts found.");
        return;
    }

    let wins = biggest_wins(report);
    if !wins.is_empty() {
        println!();
        println!("Biggest wins:");
        for (index, (project, candidate)) in wins.into_iter().enumerate() {
            let project_name = short_path(&project.path);
            println!(
                "  {}. {}/{} - {} reclaimable ({}, {}, {})",
                index + 1,
                truncate(&project_name, 44),
                candidate.name,
                format_bytes(candidate.bytes),
                format_percent(project.artifact_percent),
                candidate.safety,
                candidate.category
            );
        }
    }

    println!();
    println!(
        "{:<30} {:<13} {:<18} {:<8} {:>10} {:>7} {:<8} {:>5} Reason",
        "Project", "Kind", "Candidate", "Category", "Size", "Junk", "Safety", "Risk"
    );
    println!("{}", "-".repeat(124));

    for project in &report.projects {
        let project_name = short_path(&project.path);
        for candidate in &project.candidates {
            let reason = candidate
                .reasons
                .first()
                .or_else(|| candidate.warnings.first())
                .cloned()
                .unwrap_or_default();
            println!(
                "{:<30} {:<13} {:<18} {:<8} {:>10} {:>7} {:<8} {:>5} {}",
                truncate(&project_name, 30),
                truncate(&project.kind, 13),
                truncate(&candidate.name, 18),
                candidate.category,
                format_bytes(candidate.bytes),
                format_percent(project.artifact_percent),
                candidate.safety,
                format_risk(candidate.risk_score),
                reason
            );
        }
    }
}

fn format_risk(score: f32) -> String {
    format!("{score:.2}")
}

pub fn print_explanation(explanation: &Explanation) {
    println!("Path: {}", explanation.path.display());
    println!("Safety: {}", explanation.safety);
    if let Some(rule_id) = &explanation.rule_id {
        println!("Rule: {rule_id}");
    }
    if let Some(category) = explanation.category {
        println!("Category: {category}");
    }
    if !explanation.reasons.is_empty() {
        println!("Reasons:");
        for reason in &explanation.reasons {
            println!("  - {reason}");
        }
    }
    if !explanation.warnings.is_empty() {
        println!("Warnings:");
        for warning in &explanation.warnings {
            println!("  - {warning}");
        }
    }
    if let Some(hint) = &explanation.restore_hint {
        println!("Restore: {hint}");
    }
    if let Some(score) = explanation.risk_score {
        println!("Risk: {}", format_risk(score));
    }
    if explanation.safety == Safety::Unknown {
        println!("No built-in cleanup rule matched this path.");
    }
}

pub fn print_doctor(report: &DoctorReport) {
    println!("{:<26} {:<10} Anchor / Reason", "Rule", "Status");
    println!("{}", "-".repeat(76));
    for entry in &report.entries {
        let (status_label, detail) = match &entry.status {
            Status::Applicable => (
                "applicable",
                short_path(&entry.anchor.display().to_string()),
            ),
            Status::Skipped { reason } => ("skipped", (*reason).to_string()),
        };
        println!("{:<26} {:<10} {}", entry.rule_id, status_label, detail);
    }
    println!();
    println!(
        "{} of {} rules applicable on this machine.",
        report.applicable_count(),
        report.total_count()
    );
}

pub fn print_rules() {
    println!(
        "{:<24} {:<8} {:<18} Restore hint",
        "Rule", "Category", "Candidate"
    );
    println!("{}", "-".repeat(88));
    for rule in rules::rule_catalog() {
        println!(
            "{:<24} {:<8} {:<18} {}",
            rule.rule_id, rule.category, rule.candidate, rule.restore_hint
        );
    }
}

fn short_path(path: &str) -> String {
    for home in home_prefixes() {
        let Some(rest) = path.strip_prefix(&home) else {
            continue;
        };
        if rest.is_empty() {
            return "~".to_string();
        }
        if rest.starts_with(std::path::MAIN_SEPARATOR)
            || rest.starts_with('/')
            || rest.starts_with('\\')
        {
            return format!("~{rest}");
        }
    }
    path.to_string()
}

fn home_prefixes() -> Vec<String> {
    let mut prefixes = Vec::new();
    for key in home_env_keys() {
        let Some(home) = std::env::var_os(key) else {
            continue;
        };
        if home.is_empty() {
            continue;
        }

        let raw = std::path::PathBuf::from(home);
        push_unique(&mut prefixes, raw.display().to_string());
        if let Ok(canonical) = raw.canonicalize() {
            push_unique(&mut prefixes, canonical.display().to_string());
        }
    }
    prefixes
}

#[cfg(windows)]
fn home_env_keys() -> &'static [&'static str] {
    &["HOME", "USERPROFILE"]
}

#[cfg(not(windows))]
fn home_env_keys() -> &'static [&'static str] {
    &["HOME"]
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn biggest_wins(report: &ScanReport) -> Vec<(&ProjectReport, &Candidate)> {
    let mut wins = report
        .projects
        .iter()
        .flat_map(|project| {
            project
                .candidates
                .iter()
                .filter(|candidate| candidate.safety != Safety::Blocked && candidate.bytes > 0)
                .map(move |candidate| (project, candidate))
        })
        .collect::<Vec<_>>();

    wins.sort_by_key(|(_, candidate)| std::cmp::Reverse(candidate.bytes));
    wins.truncate(5);
    wins
}

fn format_percent(value: f64) -> String {
    format!("{value:.1}%")
}

#[cfg(feature = "graveyard")]
pub fn print_graveyard_list(records: &[crate::graveyard::ManifestRecord]) {
    if records.is_empty() {
        println!("No active graves.");
        return;
    }

    println!(
        "{:<22} {:<20} {:>10} {:<20} Original",
        "Id", "Deleted (UTC)", "Size", "Rule"
    );
    println!("{}", "-".repeat(110));
    for r in records {
        let deleted = r.deleted_at.format("%Y-%m-%d %H:%M:%S").to_string();
        println!(
            "{:<22} {:<20} {:>10} {:<20} {}",
            truncate(&r.id, 22),
            deleted,
            format_bytes(r.size_bytes),
            truncate(&r.rule_id, 20),
            r.original_path.display(),
        );
    }
}

fn truncate(value: &str, width: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= width {
        return value.to_string();
    }
    chars
        .into_iter()
        .take(width.saturating_sub(1))
        .chain(std::iter::once('~'))
        .collect()
}
