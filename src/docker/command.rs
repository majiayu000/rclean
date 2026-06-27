use std::io::Read;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;

use tracing::debug;
use wait_timeout::ChildExt;

use super::DockerStatus;

const MAX_CAPTURE_BYTES: usize = 64 * 1024;

#[derive(Debug)]
pub(super) struct DockerCommandOutput {
    pub(super) stdout: String,
}

#[derive(Debug)]
pub(super) struct DockerCommandError {
    command: String,
    kind: DockerCommandErrorKind,
}

#[derive(Debug)]
enum DockerCommandErrorKind {
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
        stdout: String,
        stderr: String,
    },
    NonZero {
        status: ExitStatus,
        stdout: String,
        stderr: String,
    },
}

pub(super) fn run_docker_command(
    program: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<DockerCommandOutput, DockerCommandError> {
    let command_label = display_docker_command(program, args);
    let stdout_file = tempfile::NamedTempFile::new().map_err(|source| DockerCommandError {
        command: command_label.clone(),
        kind: DockerCommandErrorKind::CaptureFile {
            stream: "stdout",
            source,
        },
    })?;
    let stderr_file = tempfile::NamedTempFile::new().map_err(|source| DockerCommandError {
        command: command_label.clone(),
        kind: DockerCommandErrorKind::CaptureFile {
            stream: "stderr",
            source,
        },
    })?;
    let stdout_writer = stdout_file.reopen().map_err(|source| DockerCommandError {
        command: command_label.clone(),
        kind: DockerCommandErrorKind::CaptureFile {
            stream: "stdout",
            source,
        },
    })?;
    let stderr_writer = stderr_file.reopen().map_err(|source| DockerCommandError {
        command: command_label.clone(),
        kind: DockerCommandErrorKind::CaptureFile {
            stream: "stderr",
            source,
        },
    })?;

    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::from(stdout_writer))
        .stderr(Stdio::from(stderr_writer));
    let mut child = command.spawn().map_err(|source| DockerCommandError {
        command: command_label.clone(),
        kind: DockerCommandErrorKind::Spawn(source),
    })?;

    let status = match child.wait_timeout(timeout) {
        Ok(Some(status)) => status,
        Ok(None) => {
            let kill_result = child.kill();
            let reap_result = child.wait();
            let stdout = read_capture(&stdout_file, "stdout", &command_label)?;
            let stderr = read_capture(&stderr_file, "stderr", &command_label)?;
            if let Err(source) = kill_result {
                return Err(DockerCommandError {
                    command: command_label,
                    kind: DockerCommandErrorKind::Kill(source),
                });
            }
            if let Err(source) = reap_result {
                return Err(DockerCommandError {
                    command: command_label,
                    kind: DockerCommandErrorKind::Reap(source),
                });
            }
            return Err(DockerCommandError {
                command: command_label,
                kind: DockerCommandErrorKind::TimedOut {
                    timeout,
                    stdout,
                    stderr,
                },
            });
        }
        Err(source) => {
            if let Err(kill_source) = child.kill() {
                return Err(DockerCommandError {
                    command: command_label,
                    kind: DockerCommandErrorKind::Kill(kill_source),
                });
            }
            if let Err(reap_source) = child.wait() {
                return Err(DockerCommandError {
                    command: command_label,
                    kind: DockerCommandErrorKind::Reap(reap_source),
                });
            }
            return Err(DockerCommandError {
                command: command_label,
                kind: DockerCommandErrorKind::Wait(source),
            });
        }
    };

    let stdout = read_capture(&stdout_file, "stdout", &command_label)?;
    let stderr = read_capture(&stderr_file, "stderr", &command_label)?;
    if !status.success() {
        return Err(DockerCommandError {
            command: command_label,
            kind: DockerCommandErrorKind::NonZero {
                status,
                stdout,
                stderr,
            },
        });
    }

    if !stderr.is_empty() {
        debug!(command = %command_label, stderr = %stderr, "docker command wrote stderr");
    }
    Ok(DockerCommandOutput { stdout })
}

pub(super) fn status_from_command_error(error: DockerCommandError) -> DockerStatus {
    match error.kind {
        DockerCommandErrorKind::Spawn(source) => {
            if source.kind() == std::io::ErrorKind::NotFound {
                DockerStatus::Unavailable {
                    reason: format!("Docker CLI not found while running {}", error.command),
                }
            } else {
                DockerStatus::Unavailable {
                    reason: format!("failed to start {}: {source}", error.command),
                }
            }
        }
        DockerCommandErrorKind::TimedOut {
            timeout,
            stdout,
            stderr,
        } => {
            if !stdout.is_empty() || !stderr.is_empty() {
                debug!(
                    command = %error.command,
                    stdout = %stdout,
                    stderr = %stderr,
                    "docker command timed out with partial output"
                );
            }
            DockerStatus::TimedOut {
                command: error.command,
                timeout_ms: timeout.as_millis(),
            }
        }
        DockerCommandErrorKind::NonZero {
            status,
            stdout,
            stderr,
        } => {
            let detail = command_failure_detail(status, &stdout, &stderr);
            if looks_permission_denied(&detail) {
                DockerStatus::PermissionDenied { reason: detail }
            } else {
                DockerStatus::Unavailable { reason: detail }
            }
        }
        DockerCommandErrorKind::CaptureFile { stream, source }
        | DockerCommandErrorKind::Capture { stream, source } => DockerStatus::Error {
            reason: format!("failed to capture {stream} for {}: {source}", error.command),
        },
        DockerCommandErrorKind::Wait(source)
        | DockerCommandErrorKind::Kill(source)
        | DockerCommandErrorKind::Reap(source) => DockerStatus::Error {
            reason: format!("failed while waiting for {}: {source}", error.command),
        },
    }
}

fn read_capture(
    file: &tempfile::NamedTempFile,
    stream: &'static str,
    command_label: &str,
) -> Result<String, DockerCommandError> {
    let mut reader = file.reopen().map_err(|source| DockerCommandError {
        command: command_label.to_string(),
        kind: DockerCommandErrorKind::Capture { stream, source },
    })?;
    let mut buffer = Vec::new();
    reader
        .by_ref()
        .take(MAX_CAPTURE_BYTES as u64)
        .read_to_end(&mut buffer)
        .map_err(|source| DockerCommandError {
            command: command_label.to_string(),
            kind: DockerCommandErrorKind::Capture { stream, source },
        })?;
    Ok(String::from_utf8_lossy(&buffer).trim().to_string())
}

fn command_failure_detail(status: ExitStatus, stdout: &str, stderr: &str) -> String {
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        "no output"
    };
    format!("docker command exited with {status}: {detail}")
}

fn looks_permission_denied(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower.contains("permission denied")
        || lower.contains("access is denied")
        || (lower.contains("permission") && lower.contains("docker daemon"))
}

fn display_docker_command(program: &Path, args: &[&str]) -> String {
    let mut parts = vec![program.display().to_string()];
    parts.extend(args.iter().map(|arg| (*arg).to_string()));
    parts.join(" ")
}
