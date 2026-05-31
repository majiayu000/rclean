use std::env;

use super::types::{AgentProcess, AgentTool};

pub(super) fn parse_agent_processes(tool: AgentTool, output: &str) -> Vec<AgentProcess> {
    output
        .lines()
        .filter_map(parse_process_line)
        .filter(|process| process_matches_tool(tool, &process.command))
        .map(|process| AgentProcess {
            pid: process.pid,
            command: compact_command(&redact_command(&process.command)),
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

pub(super) fn process_matches_tool(tool: AgentTool, command: &str) -> bool {
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

fn redact_command(command: &str) -> String {
    let mut tokens = Vec::new();
    let mut redact_next = false;

    for token in command.split_whitespace() {
        if redact_next {
            if normalize_secret_key(token) == "bearer" {
                tokens.push(token.to_string());
            } else {
                tokens.push("<redacted>".to_string());
                redact_next = false;
            }
            continue;
        }

        let (redacted, expects_value) = redact_command_token(token);
        tokens.push(redacted);
        redact_next = expects_value;
    }

    redact_home_path(&tokens.join(" "))
}

fn redact_command_token(token: &str) -> (String, bool) {
    if is_secret_value(token) {
        return ("<redacted>".to_string(), false);
    }

    if let Some((key, _value)) = token.split_once('=')
        && is_sensitive_key(key)
    {
        return (format!("{key}=<redacted>"), false);
    }

    if expects_sensitive_value(token) {
        return (token.to_string(), true);
    }

    (redact_home_path(token), false)
}

fn expects_sensitive_value(token: &str) -> bool {
    let key = normalize_secret_key(token);
    key == "bearer" || is_sensitive_key(&key)
}

fn is_sensitive_key(key: &str) -> bool {
    let key = normalize_secret_key(key);
    key.contains("api-key")
        || key.contains("apikey")
        || key.contains("authorization")
        || key.contains("credential")
        || key.contains("password")
        || key.contains("secret")
        || key.contains("session")
        || key.contains("token")
        || key == "auth"
        || key == "cookie"
}

fn normalize_secret_key(key: &str) -> String {
    key.trim_start_matches('-')
        .trim_matches(|c: char| matches!(c, '"' | '\'' | ':' | ';'))
        .to_ascii_lowercase()
        .replace('_', "-")
}

fn is_secret_value(value: &str) -> bool {
    let value = value.trim_matches(|c: char| matches!(c, '"' | '\'' | ',' | ';'));
    value.starts_with("sk-")
        || value.starts_with("gho_")
        || value.starts_with("ghp_")
        || value.starts_with("github_pat_")
        || value.starts_with("glpat-")
        || value.starts_with("xoxb-")
        || value.starts_with("xoxp-")
}

pub(super) fn redact_home_path(value: &str) -> String {
    let Some(home) = env::var_os("HOME")
        .and_then(|home| home.into_string().ok())
        .filter(|home| !home.is_empty())
    else {
        return value.to_string();
    };

    value.replace(&home, "~")
}
