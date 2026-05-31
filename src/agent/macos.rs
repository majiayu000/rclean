use super::process::process_matches_tool;
use super::types::{AgentTool, PowerAssertion, PowerReport};

pub(super) fn parse_pmset_assertions(tool: AgentTool, output: &str) -> PowerReport {
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

fn parse_pmset_summary_bool(output: &str, key: &str) -> Option<bool> {
    output.lines().find_map(|line| {
        let trimmed = line.trim();
        if !trimmed.starts_with(key) {
            return None;
        }
        trimmed.split_whitespace().last().map(|value| value == "1")
    })
}

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

pub(super) fn parse_defaults_bool(output: &str, key: &str) -> Option<bool> {
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

pub(super) fn parse_defaults_string(output: &str, key: &str) -> Option<String> {
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
