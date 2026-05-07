use crate::model::{Candidate, Explanation, ProjectReport, Safety, ScanReport, format_bytes};
use crate::rules;

pub fn print_json(report: &ScanReport) -> Result<(), String> {
    let json = serde_json::to_string_pretty(report)
        .map_err(|err| format!("failed to serialize JSON report: {err}"))?;
    println!("{json}");
    Ok(())
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
        "{:<30} {:<13} {:<18} {:<8} {:>10} {:>7} {:<8} Reason",
        "Project", "Kind", "Candidate", "Category", "Size", "Junk", "Safety"
    );
    println!("{}", "-".repeat(118));

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
                "{:<30} {:<13} {:<18} {:<8} {:>10} {:>7} {:<8} {}",
                truncate(&project_name, 30),
                truncate(&project.kind, 13),
                truncate(&candidate.name, 18),
                candidate.category,
                format_bytes(candidate.bytes),
                format_percent(project.artifact_percent),
                candidate.safety,
                reason
            );
        }
    }
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
    if explanation.safety == Safety::Unknown {
        println!("No built-in cleanup rule matched this path.");
    }
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
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() && path.starts_with(&home) {
        return path.replacen(&home, "~", 1);
    }
    path.to_string()
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
