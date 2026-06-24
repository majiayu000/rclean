#![cfg(not(windows))]

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn pip_cache_clean_uses_pip_cache_purge() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let pip_cache = temp.path().join(".cache").join("pip");
    std::fs::create_dir_all(&pip_cache)?;
    std::fs::write(pip_cache.join("blob"), "x")?;

    let fake_bin = temp.path().join("bin");
    std::fs::create_dir(&fake_bin)?;
    let fake_pip_output = temp.path().join("fake-pip-output.txt");
    write_fake_pip(&fake_bin)?;

    let path = path_with_fake_pip(&fake_bin)?;
    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", temp.path())
        .env("PATH", path)
        .env("FAKE_PIP_OUT", &fake_pip_output)
        .args([
            "clean",
            "--home",
            "--all",
            "--permanent",
            "--yes",
            "--min-size",
            "0",
            "--rule",
            "pip.cache",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleaned: 1 candidates"));

    let fake_output = std::fs::read_to_string(fake_pip_output)?;
    assert!(fake_output.contains("cache"));
    assert!(fake_output.contains("purge"));
    let normalized_output = fake_output.replace('\\', "/");
    let normalized_pip_cache = pip_cache.display().to_string().replace('\\', "/");
    assert!(
        normalized_output.contains(&format!("PIP_CACHE_DIR={normalized_pip_cache}"))
            || normalized_output.contains("PIP_CACHE_DIR=")
                && normalized_output.contains(".cache/pip")
    );
    Ok(())
}

#[test]
fn pip_cache_trash_mode_does_not_execute_pip() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let pip_cache = temp.path().join(".cache").join("pip");
    std::fs::create_dir_all(&pip_cache)?;
    std::fs::write(pip_cache.join("blob"), "x")?;

    let fake_bin = temp.path().join("bin");
    std::fs::create_dir(&fake_bin)?;
    let fake_pip_output = temp.path().join("fake-pip-output.txt");
    write_fake_pip(&fake_bin)?;

    let path = path_with_fake_pip(&fake_bin)?;
    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", temp.path())
        .env("PATH", path)
        .env("FAKE_PIP_OUT", &fake_pip_output)
        .args([
            "clean",
            "--home",
            "--all",
            "--yes",
            "--min-size",
            "0",
            "--rule",
            "pip.cache",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("mode: trash"))
        .stdout(predicate::str::contains("Failed: 1"))
        .stdout(predicate::str::contains("pip cache cleanup"))
        .stdout(predicate::str::contains("rerun with --permanent"));

    assert!(
        !fake_pip_output.exists(),
        "trash-mode cleanup must not execute pip cache purge"
    );
    Ok(())
}

fn path_with_fake_pip(
    fake_bin: &std::path::Path,
) -> Result<std::ffi::OsString, std::env::JoinPathsError> {
    let mut paths = vec![fake_bin.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths)
}

fn write_fake_pip(fake_bin: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let path = fake_bin.join("pip");
    std::fs::write(
        &path,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$FAKE_PIP_OUT\"\nprintf 'PIP_CACHE_DIR=%s\\n' \"$PIP_CACHE_DIR\" >> \"$FAKE_PIP_OUT\"\n",
    )?;
    let mut permissions = std::fs::metadata(&path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)
}
