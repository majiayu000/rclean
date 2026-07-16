use crate::agent::{AgentReport, OptimizeResult};
use crate::doctor::{DoctorReport, Status};
use crate::error::RcleanError;
use crate::model::{Candidate, Explanation, ProjectReport, Safety, ScanReport, format_bytes};
use crate::rules;
use crate::stdio::outln;

pub fn print_json(report: &ScanReport) -> Result<(), RcleanError> {
    let json = serde_json::to_string_pretty(report)?;
    outln!("{json}");
    Ok(())
}

pub fn print_agent_report(report: &AgentReport) -> Result<(), RcleanError> {
    outln!("Agent: {}", report.tool);
    outln!("Generated: {}", report.generated_at);

    outln!();
    outln!("Power:");
    if report.power.supported {
        outln!(
            "  display sleep assertion: {}",
            format_bool(report.power.prevent_user_idle_display_sleep)
        );
        outln!(
            "  idle system sleep assertion: {}",
            format_bool(report.power.prevent_user_idle_system_sleep)
        );
        outln!(
            "  agent blocks display sleep: {}",
            if report.power.agent_blocks_display_sleep {
                "yes"
            } else {
                "no"
            }
        );
        for assertion in &report.power.assertions {
            outln!(
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
        outln!("  unsupported on this platform");
    }

    outln!();
    outln!("Disk:");
    for entry in &report.disk {
        let size = if entry.exists {
            format_bytes(entry.bytes)
        } else {
            "missing".to_string()
        };
        outln!("  {:<28} {:>8}  {}", entry.label, size, entry.path);
    }

    outln!();
    outln!("Processes:");
    if report.processes.is_empty() {
        outln!("  none detected");
    } else {
        let visible_processes = 20;
        for process in report.processes.iter().take(visible_processes) {
            outln!("  - pid {} {}", process.pid, process.command);
        }
        if report.processes.len() > visible_processes {
            outln!(
                "  ... and {} more process(es)",
                report.processes.len() - visible_processes
            );
        }
    }

    outln!();
    outln!("Auto update:");
    if report.auto_update.supported {
        outln!(
            "  automatically update: {}",
            format_bool(report.auto_update.automatically_update)
        );
        outln!(
            "  automatic checks: {}",
            format_bool(report.auto_update.automatic_checks)
        );
        if let Some(last_check) = &report.auto_update.last_check_time {
            outln!("  last check: {last_check}");
        }
    } else {
        outln!("  unsupported on this platform");
    }

    if !report.warnings.is_empty() {
        outln!();
        outln!("Warnings:");
        for warning in &report.warnings {
            outln!("  - {warning}");
        }
    }

    if !report.recommendations.is_empty() {
        outln!();
        outln!("Recommendations:");
        for recommendation in &report.recommendations {
            outln!("  - {recommendation}");
        }
    }
    Ok(())
}

pub fn print_agent_optimize_result(result: &OptimizeResult) -> Result<(), RcleanError> {
    outln!("Agent: {}", result.tool);
    outln!(
        "Mode: {}",
        if result.applied { "applied" } else { "dry-run" }
    );
    for action in &result.actions {
        outln!();
        outln!("{}: {}", action.id, action.status);
        outln!("  {}", action.description);
        for command in &action.commands {
            outln!("  $ {command}");
        }
    }
    if !result.applied {
        outln!();
        outln!("Re-run with --yes to apply the selected changes.");
    }
    Ok(())
}

fn format_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "on",
        Some(false) => "off",
        None => "unknown",
    }
}

pub fn print_table(report: &ScanReport) -> Result<(), RcleanError> {
    outln!();
    outln!(
        "Summary: {} projects scanned, {} candidates, {} reclaimable",
        report.summary.projects_scanned,
        report.summary.candidates,
        format_bytes(report.summary.total_bytes)
    );
    outln!(
        "Safety: {} safe, {} caution, {} blocked, {} report-only",
        report.summary.safe_candidates,
        report.summary.caution_candidates,
        report.summary.blocked_candidates,
        report.summary.report_only_candidates
    );
    if let Some(disk) = &report.disk_attribution {
        print_disk_attribution(disk)?;
    }
    print_scan_warnings(&report.warnings)?;

    if report.projects.is_empty() {
        outln!("No cleanable developer artifacts found.");
        print_empty_scan_hint()?;
        return Ok(());
    }

    let wins = biggest_wins(report);
    if !wins.is_empty() {
        outln!();
        outln!("Biggest wins:");
        for (index, (project, candidate)) in wins.into_iter().enumerate() {
            let project_name = short_path(&project.path);
            let staleness = match candidate.staleness_days {
                Some(days) if days >= report.stale_after_days => {
                    format!(", untouched {days}d")
                }
                _ => String::new(),
            };
            outln!(
                "  {}. {}/{} - {} reclaimable ({}, {}, {}{})",
                index + 1,
                truncate(&project_name, 44),
                candidate.name,
                format_bytes(candidate.bytes),
                format_percent(project.artifact_percent),
                candidate.safety,
                candidate.category,
                staleness
            );
        }
    }

    outln!();
    outln!(
        "{:<30} {:<13} {:<18} {:<8} {:>10} {:>7} {:<8} {:>5} {:>6} Reason",
        "Project",
        "Kind",
        "Candidate",
        "Category",
        "Size",
        "Junk",
        "Safety",
        "Risk",
        "Stale"
    );
    outln!("{}", "-".repeat(131));

    for project in &report.projects {
        let project_name = short_path(&project.path);
        for candidate in &project.candidates {
            let reason = candidate
                .reasons
                .first()
                .or_else(|| candidate.warnings.first())
                .cloned()
                .unwrap_or_default();
            outln!(
                "{:<30} {:<13} {:<18} {:<8} {:>10} {:>7} {:<8} {:>5} {:>6} {}",
                truncate(&project_name, 30),
                truncate(&project.kind, 13),
                truncate(&candidate.name, 18),
                candidate.category,
                format_bytes(candidate.bytes),
                format_percent(project.artifact_percent),
                candidate.safety,
                format_risk(candidate.risk_score),
                format_staleness(candidate.staleness_days),
                reason
            );
        }
    }
    Ok(())
}

fn print_empty_scan_hint() -> Result<(), RcleanError> {
    outln!(
        "Hint: try `rclean scan --home` for toolchain caches or `rclean scan --tmp` for temp worktrees."
    );
    Ok(())
}

fn print_scan_warnings(warnings: &[crate::model::ScanWarning]) -> Result<(), RcleanError> {
    if warnings.is_empty() {
        return Ok(());
    }

    outln!();
    outln!("Warnings during scan:");
    for warning in warnings {
        outln!("  - {warning}");
    }
    outln!(
        "{} warning(s) during scan. Results may be incomplete.",
        warnings.len()
    );
    Ok(())
}

fn print_disk_attribution(disk: &crate::model::DiskAttribution) -> Result<(), RcleanError> {
    outln!();
    outln!("Disk attribution:");
    if let Some(container) = &disk.apfs_container {
        outln!(
            "  APFS container: {} used / {} total ({} free)",
            format_optional_bytes(container.used_bytes),
            format_optional_bytes(container.capacity_bytes),
            format_optional_bytes(container.free_bytes)
        );
    }
    if let Some(system) = &disk.system_volume {
        outln!("  System volume: {} used", format_bytes(system.used_bytes));
    }
    if let Some(data) = &disk.data_volume {
        outln!("  Data volume: {} used", format_bytes(data.used_bytes));
    }
    if !disk.data_contributors.is_empty() {
        outln!("  Top Data contributors:");
        for contributor in &disk.data_contributors {
            outln!(
                "    {:<14} {:>8}  {}",
                contributor.label,
                contributor
                    .bytes
                    .map(format_bytes)
                    .unwrap_or_else(|| "unknown".to_string()),
                contributor.path
            );
        }
    }
    for warning in &disk.warnings {
        outln!("  warning: {warning}");
    }
    Ok(())
}

fn format_optional_bytes(bytes: Option<u64>) -> String {
    bytes
        .map(format_bytes)
        .unwrap_or_else(|| "unknown".to_string())
}

fn format_risk(score: f32) -> String {
    format!("{score:.2}")
}

pub fn print_explanation(explanation: &Explanation) -> Result<(), RcleanError> {
    outln!("Path: {}", explanation.path.display());
    outln!("Safety: {}", explanation.safety);
    if let Some(rule_id) = &explanation.rule_id {
        outln!("Rule: {rule_id}");
    }
    if let Some(category) = explanation.category {
        outln!("Category: {category}");
    }
    if !explanation.reasons.is_empty() {
        outln!("Reasons:");
        for reason in &explanation.reasons {
            outln!("  - {reason}");
        }
    }
    if !explanation.warnings.is_empty() {
        outln!("Warnings:");
        for warning in &explanation.warnings {
            outln!("  - {warning}");
        }
    }
    if let Some(hint) = &explanation.restore_hint {
        outln!("Restore: {hint}");
    }
    if let Some(score) = explanation.risk_score {
        outln!("Risk: {}", format_risk(score));
    }
    if explanation.safety == Safety::Unknown {
        outln!("No built-in cleanup rule matched this path.");
    }
    Ok(())
}

pub fn print_doctor(report: &DoctorReport) -> Result<(), RcleanError> {
    outln!("{:<26} {:<10} Anchor / Reason", "Rule", "Status");
    outln!("{}", "-".repeat(76));
    for entry in &report.entries {
        let (status_label, detail) = match &entry.status {
            Status::Applicable => (
                "applicable",
                short_path(&entry.anchor.display().to_string()),
            ),
            Status::Skipped { reason } => ("skipped", reason.clone()),
        };
        outln!("{:<26} {:<10} {}", entry.rule_id, status_label, detail);
    }
    outln!();
    outln!(
        "{} of {} rules applicable on this machine.",
        report.applicable_count(),
        report.total_count()
    );
    Ok(())
}

pub fn print_rules() -> Result<(), RcleanError> {
    outln!(
        "{:<24} {:<8} {:<18} Restore hint",
        "Rule",
        "Category",
        "Candidate"
    );
    outln!("{}", "-".repeat(88));
    for rule in rules::rule_catalog() {
        outln!(
            "{:<24} {:<8} {:<18} {}",
            rule.rule_id,
            rule.category,
            rule.candidate,
            rule.restore_hint
        );
    }
    Ok(())
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

pub(crate) fn format_staleness(days: Option<u64>) -> String {
    match days {
        Some(days) => format!("{days}d"),
        None => "-".to_string(),
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
                .filter(|candidate| {
                    candidate.safety != Safety::Blocked
                        && candidate.safety != Safety::ReportOnly
                        && candidate.bytes > 0
                })
                .map(move |candidate| (project, candidate))
        })
        .collect::<Vec<_>>();

    // Stale candidates first (nothing touched the project for at least
    // `stale_after_days`), then by size within each group: an old 2 GB
    // target is a better win than a 3 GB one rebuilt this morning.
    let threshold = report.stale_after_days;
    wins.sort_by_key(|(_, candidate)| {
        let stale = candidate
            .staleness_days
            .is_some_and(|days| days >= threshold);
        (std::cmp::Reverse(stale), std::cmp::Reverse(candidate.bytes))
    });
    wins.truncate(5);
    wins
}

fn format_percent(value: f64) -> String {
    format!("{value:.1}%")
}

#[cfg(feature = "graveyard")]
pub fn print_graveyard_list(
    records: &[crate::graveyard::ManifestRecord],
) -> Result<(), RcleanError> {
    if records.is_empty() {
        outln!("No active graves.");
        return Ok(());
    }

    outln!(
        "{:<22} {:<20} {:>10} {:<20} Original",
        "Id",
        "Deleted (UTC)",
        "Size",
        "Rule"
    );
    outln!("{}", "-".repeat(110));
    for r in records {
        let deleted = r.deleted_at.format("%Y-%m-%d %H:%M:%S").to_string();
        outln!(
            "{:<22} {:<20} {:>10} {:<20} {}",
            truncate(&r.id, 22),
            deleted,
            format_bytes(r.size_bytes),
            truncate(&r.rule_id, 20),
            r.original_path.display(),
        );
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ActivityInfo, Category, ProjectReport, Summary};

    fn candidate(name: &str, bytes: u64, staleness_days: Option<u64>) -> Candidate {
        Candidate {
            path: format!("/tmp/proj/{name}"),
            name: name.to_string(),
            rule_id: "rust.target".to_string(),
            category: Category::Build,
            bytes,
            safety: Safety::Safe,
            requires_sudo: false,
            reasons: vec!["test".to_string()],
            warnings: Vec::new(),
            restore_hint: "cargo build".to_string(),
            risk_score: 0.1,
            staleness_days,
        }
    }

    fn report_with(candidates: Vec<Candidate>) -> ScanReport {
        ScanReport {
            schema_version: 1,
            tool_version: "test".to_string(),
            scanned_at: "2026-07-03T00:00:00Z".to_string(),
            roots: vec!["/tmp".to_string()],
            disk_attribution: None,
            warnings: Vec::new(),
            stale_after_days: 30,
            summary: Summary {
                projects_scanned: 1,
                projects_with_candidates: 1,
                candidates: candidates.len(),
                safe_candidates: candidates.len(),
                caution_candidates: 0,
                blocked_candidates: 0,
                report_only_candidates: 0,
                total_bytes: candidates.iter().map(|c| c.bytes).sum(),
            },
            projects: vec![ProjectReport {
                path: "/tmp/proj".to_string(),
                kind: "Rust".to_string(),
                markers: vec!["Cargo.toml".to_string()],
                git: None,
                activity: ActivityInfo {
                    last_modified: "2026-05-01T00:00:00Z".to_string(),
                    source: "test".to_string(),
                },
                total_bytes: candidates.iter().map(|c| c.bytes).sum(),
                project_bytes: 100,
                artifact_percent: 50.0,
                candidates,
            }],
        }
    }

    #[test]
    fn biggest_wins_ranks_stale_candidates_before_larger_fresh_ones() {
        let report = report_with(vec![
            candidate("fresh-large", 3_000_000_000, Some(0)),
            candidate("stale-small", 2_000_000_000, Some(94)),
        ]);
        let wins = biggest_wins(&report);
        assert_eq!(wins[0].1.name, "stale-small");
        assert_eq!(wins[1].1.name, "fresh-large");
    }

    #[test]
    fn biggest_wins_falls_back_to_size_within_the_same_staleness_group() {
        let report = report_with(vec![
            candidate("stale-small", 1_000, Some(40)),
            candidate("stale-large", 2_000, Some(35)),
        ]);
        let wins = biggest_wins(&report);
        assert_eq!(wins[0].1.name, "stale-large");
    }

    #[test]
    fn format_staleness_renders_days_or_dash() {
        assert_eq!(format_staleness(Some(94)), "94d");
        assert_eq!(format_staleness(None), "-");
    }
}
