use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn go_module_cache_clean_uses_go_clean_modcache() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let module_cache = temp.path().join("go").join("pkg").join("mod");
    let download = module_cache.join("cache").join("download");
    std::fs::create_dir_all(&download)?;
    std::fs::write(download.join("blob"), "x")?;

    let fake_bin = temp.path().join("bin");
    std::fs::create_dir(&fake_bin)?;
    let fake_go_output = temp.path().join("fake-go-output.txt");
    write_fake_go(&fake_bin)?;

    let path = path_with_fake_go(&fake_bin)?;
    let mut cmd = Command::cargo_bin("rclean")?;
    cmd.env("HOME", temp.path())
        .env("PATH", path)
        .env("FAKE_GO_OUT", &fake_go_output)
        .args([
            "clean",
            "--home",
            "--all",
            "--include-caution",
            "--permanent",
            "--yes",
            "--min-size",
            "0",
            "--rule",
            "go.module_cache",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleaned: 1 candidates"));

    let fake_output = std::fs::read_to_string(fake_go_output)?;
    assert!(fake_output.contains("clean"));
    assert!(fake_output.contains("-modcache"));
    let normalized_output = fake_output.replace('\\', "/");
    let normalized_module_cache = module_cache.display().to_string().replace('\\', "/");
    assert!(
        normalized_output.contains(&format!("GOMODCACHE={normalized_module_cache}"))
            || normalized_output.contains("GOMODCACHE=")
                && normalized_output.contains("go/pkg/mod")
    );
    Ok(())
}

fn path_with_fake_go(
    fake_bin: &std::path::Path,
) -> Result<std::ffi::OsString, std::env::JoinPathsError> {
    let mut paths = vec![fake_bin.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths)
}

#[cfg(unix)]
fn write_fake_go(fake_bin: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let path = fake_bin.join("go");
    std::fs::write(
        &path,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$FAKE_GO_OUT\"\nprintf 'GOMODCACHE=%s\\n' \"$GOMODCACHE\" >> \"$FAKE_GO_OUT\"\n",
    )?;
    let mut permissions = std::fs::metadata(&path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)
}

#[cfg(windows)]
fn write_fake_go(fake_bin: &std::path::Path) -> std::io::Result<()> {
    std::fs::write(
        fake_bin.join("go.cmd"),
        "@echo off\r\n> \"%FAKE_GO_OUT%\" (\r\n  echo %1\r\n  echo %2\r\n  echo GOMODCACHE=%GOMODCACHE%\r\n)\r\nexit /b 0\r\n",
    )
}
