use std::ffi::OsStr;
use std::fmt;
use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;

use wait_timeout::ChildExt;

const MAX_CAPTURE_BYTES: usize = 64 * 1024;

pub(super) struct NativeToolCommand<'a> {
    pub(super) program: &'a str,
    pub(super) args: &'a [&'a str],
    pub(super) envs: &'a [(&'a str, &'a OsStr)],
    pub(super) timeout: Duration,
}

#[derive(Debug)]
pub(super) struct NativeToolError {
    command: String,
    kind: NativeToolErrorKind,
}

#[derive(Debug)]
enum NativeToolErrorKind {
    CaptureFile {
        stream: &'static str,
        source: std::io::Error,
    },
    Spawn(std::io::Error),
    Wait(std::io::Error),
    Kill(std::io::Error),
    Reap(std::io::Error),
    Capture {
        stream: &'static str,
        source: std::io::Error,
    },
    TimedOut {
        timeout: Duration,
        stdout: CapturedOutput,
        stderr: CapturedOutput,
    },
    NonZero {
        status: ExitStatus,
        stdout: CapturedOutput,
        stderr: CapturedOutput,
    },
}

#[derive(Debug)]
struct CapturedOutput {
    text: String,
    truncated: bool,
}

pub(super) fn run_native_tool(invocation: NativeToolCommand<'_>) -> Result<(), NativeToolError> {
    let command_label = display_command(invocation.program, invocation.args);
    let stdout_file = tempfile::NamedTempFile::new().map_err(|source| {
        NativeToolError::new(
            command_label.clone(),
            NativeToolErrorKind::CaptureFile {
                stream: "stdout",
                source,
            },
        )
    })?;
    let stderr_file = tempfile::NamedTempFile::new().map_err(|source| {
        NativeToolError::new(
            command_label.clone(),
            NativeToolErrorKind::CaptureFile {
                stream: "stderr",
                source,
            },
        )
    })?;
    let stdout_writer = stdout_file.reopen().map_err(|source| {
        NativeToolError::new(
            command_label.clone(),
            NativeToolErrorKind::CaptureFile {
                stream: "stdout",
                source,
            },
        )
    })?;
    let stderr_writer = stderr_file.reopen().map_err(|source| {
        NativeToolError::new(
            command_label.clone(),
            NativeToolErrorKind::CaptureFile {
                stream: "stderr",
                source,
            },
        )
    })?;

    let mut command = Command::new(invocation.program);
    command
        .args(invocation.args)
        .stdout(Stdio::from(stdout_writer))
        .stderr(Stdio::from(stderr_writer));
    for (key, value) in invocation.envs {
        command.env(key, value);
    }

    let mut child = command.spawn().map_err(|source| {
        NativeToolError::new(command_label.clone(), NativeToolErrorKind::Spawn(source))
    })?;

    let status = match child.wait_timeout(invocation.timeout) {
        Ok(Some(status)) => status,
        Ok(None) => {
            let kill_result = child.kill();
            let reap_result = child.wait();
            let stdout = read_capture(&stdout_file, "stdout", &command_label)?;
            let stderr = read_capture(&stderr_file, "stderr", &command_label)?;
            if let Err(source) = kill_result {
                return Err(NativeToolError::new(
                    command_label,
                    NativeToolErrorKind::Kill(source),
                ));
            }
            if let Err(source) = reap_result {
                return Err(NativeToolError::new(
                    command_label,
                    NativeToolErrorKind::Reap(source),
                ));
            }
            return Err(NativeToolError::new(
                command_label,
                NativeToolErrorKind::TimedOut {
                    timeout: invocation.timeout,
                    stdout,
                    stderr,
                },
            ));
        }
        Err(source) => {
            if let Err(kill_source) = child.kill() {
                return Err(NativeToolError::new(
                    command_label,
                    NativeToolErrorKind::Kill(kill_source),
                ));
            }
            if let Err(reap_source) = child.wait() {
                return Err(NativeToolError::new(
                    command_label,
                    NativeToolErrorKind::Reap(reap_source),
                ));
            }
            return Err(NativeToolError::new(
                command_label,
                NativeToolErrorKind::Wait(source),
            ));
        }
    };

    let stdout_capture = read_capture(&stdout_file, "stdout", &command_label)?;
    let stderr_capture = read_capture(&stderr_file, "stderr", &command_label)?;
    if !status.success() {
        return Err(NativeToolError::new(
            command_label,
            NativeToolErrorKind::NonZero {
                status,
                stdout: stdout_capture,
                stderr: stderr_capture,
            },
        ));
    }

    Ok(())
}

fn read_capture(
    file: &tempfile::NamedTempFile,
    stream: &'static str,
    command_label: &str,
) -> Result<CapturedOutput, NativeToolError> {
    let reader = file.reopen().map_err(|source| {
        NativeToolError::new(
            command_label.to_string(),
            NativeToolErrorKind::Capture { stream, source },
        )
    })?;
    let mut limited = reader.take((MAX_CAPTURE_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    limited.read_to_end(&mut bytes).map_err(|source| {
        NativeToolError::new(
            command_label.to_string(),
            NativeToolErrorKind::Capture { stream, source },
        )
    })?;
    let truncated = bytes.len() > MAX_CAPTURE_BYTES;
    bytes.truncate(MAX_CAPTURE_BYTES);
    Ok(CapturedOutput {
        text: String::from_utf8_lossy(&bytes).to_string(),
        truncated,
    })
}

fn display_command(program: &str, args: &[&str]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(program.to_string());
    parts.extend(args.iter().map(|arg| (*arg).to_string()));
    parts.join(" ")
}

fn format_capture(label: &str, output: &CapturedOutput) -> String {
    let trimmed = output.text.trim();
    if trimmed.is_empty() {
        format!("{label}: <empty>")
    } else if output.truncated {
        format!("{label}: {trimmed} [truncated]")
    } else {
        format!("{label}: {trimmed}")
    }
}

impl NativeToolError {
    fn new(command: String, kind: NativeToolErrorKind) -> Self {
        Self { command, kind }
    }
}

impl fmt::Display for NativeToolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            NativeToolErrorKind::CaptureFile { stream, source } => {
                write!(
                    formatter,
                    "failed to prepare {stream} capture for `{}`: {source}",
                    self.command
                )
            }
            NativeToolErrorKind::Spawn(source) => {
                write!(formatter, "failed to start `{}`: {source}", self.command)
            }
            NativeToolErrorKind::Wait(source) => {
                write!(
                    formatter,
                    "failed while waiting for `{}`: {source}",
                    self.command
                )
            }
            NativeToolErrorKind::Kill(source) => {
                write!(
                    formatter,
                    "`{}` could not be killed after failed wait or timeout: {source}",
                    self.command
                )
            }
            NativeToolErrorKind::Reap(source) => {
                write!(
                    formatter,
                    "`{}` could not be reaped after failed wait or timeout: {source}",
                    self.command
                )
            }
            NativeToolErrorKind::Capture { stream, source } => {
                write!(
                    formatter,
                    "failed to capture {stream} from `{}`: {source}",
                    self.command
                )
            }
            NativeToolErrorKind::TimedOut {
                timeout,
                stdout,
                stderr,
            } => write!(
                formatter,
                "`{}` timed out after {} ms; {}; {}",
                self.command,
                timeout.as_millis(),
                format_capture("stdout", stdout),
                format_capture("stderr", stderr)
            ),
            NativeToolErrorKind::NonZero {
                status,
                stdout,
                stderr,
            } => write!(
                formatter,
                "`{}` exited with {status}; {}; {}",
                self.command,
                format_capture("stdout", stdout),
                format_capture("stderr", stderr)
            ),
        }
    }
}

impl std::error::Error for NativeToolError {}
