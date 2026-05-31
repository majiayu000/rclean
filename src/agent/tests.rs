use std::env;

use super::macos::{parse_defaults_bool, parse_defaults_string, parse_pmset_assertions};
use super::process::parse_agent_processes;
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

#[test]
fn redacts_agent_process_command_secrets() {
    let home = env::var("HOME").unwrap_or_else(|_| "/Users/example".to_string());
    let output = format!(
        "  104 OPENAI_API_KEY=sk-test node {home}/.local/bin/codex --api-key sk-live --token=gho_secret --authorization Bearer opaque-secret\n"
    );

    let processes = parse_agent_processes(AgentTool::Codex, &output);

    assert_eq!(processes.len(), 1);
    let command = &processes[0].command;
    assert!(command.contains("OPENAI_API_KEY=<redacted>"));
    assert!(command.contains("--api-key <redacted>"));
    assert!(command.contains("--token=<redacted>"));
    assert!(command.contains("--authorization Bearer <redacted>"));
    assert!(command.contains("<redacted>"));
    assert!(command.contains("codex"));
    assert!(!command.contains("sk-test"));
    assert!(!command.contains("sk-live"));
    assert!(!command.contains("gho_secret"));
    assert!(!command.contains("opaque-secret"));
}
