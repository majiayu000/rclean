use std::process::Command;

use super::types::AgentError;

pub(super) fn run_output(program: &str, args: &[&str]) -> Result<String, AgentError> {
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
pub(super) fn run_status(program: &str, args: &[&str]) -> Result<(), AgentError> {
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
