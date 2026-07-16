use std::io;
use std::process::{Command, Stdio};

use assert_cmd::prelude::CommandCargoExt;
use tempfile::TempDir;

fn run_with_closed_stdout(
    args: &[&str],
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let (reader, writer) = io::pipe()?;
    drop(reader);

    let mut command = Command::cargo_bin("rclean")?;
    command
        .args(args)
        .stdout(Stdio::from(writer))
        .stderr(Stdio::piped());
    Ok(command.output()?)
}

fn assert_no_output_panic(output: &std::process::Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked at") && !stderr.contains("failed printing to stdout"),
        "closed stdout must not trigger a panic; stderr: {stderr}"
    );
}

#[test]
fn rules_exits_cleanly_when_stdout_reader_is_closed() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_with_closed_stdout(&["rules"])?;

    assert_eq!(output.status.code(), Some(0));
    assert_no_output_panic(&output);
    Ok(())
}

#[test]
fn doctor_exits_cleanly_when_stdout_reader_is_closed() -> Result<(), Box<dyn std::error::Error>> {
    let expected_status = Command::cargo_bin("rclean")?.arg("doctor").output()?.status;
    let output = run_with_closed_stdout(&["doctor"])?;

    assert_eq!(output.status, expected_status);
    assert_no_output_panic(&output);
    Ok(())
}

#[test]
fn clean_stops_before_delete_when_stdout_reader_is_closed() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = TempDir::new()?;
    std::fs::write(temp.path().join("package.json"), b"{}")?;
    let candidate = temp.path().join("node_modules");
    std::fs::create_dir(&candidate)?;
    std::fs::write(candidate.join("artifact.bin"), b"rebuildable")?;

    let root = temp.path().to_string_lossy().into_owned();
    let output = run_with_closed_stdout(&[
        "clean",
        &root,
        "--all",
        "--yes",
        "--permanent",
        "--min-size",
        "0",
    ])?;

    assert_eq!(output.status.code(), Some(0));
    assert_no_output_panic(&output);
    assert!(
        candidate.exists(),
        "closed pre-delete stdout must stop clean"
    );
    Ok(())
}
