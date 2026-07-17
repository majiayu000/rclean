use super::*;
use crate::model::{Category, Safety};
use std::sync::{Mutex, MutexGuard};
use tempfile::TempDir;

const FAKE_GO_TEST_TIMEOUT: Duration = Duration::from_secs(30);
static FAKE_GO_TEST_LOCK: Mutex<()> = Mutex::new(());

fn fake_go_test_guard() -> Result<MutexGuard<'static, ()>, Box<dyn std::error::Error>> {
    FAKE_GO_TEST_LOCK.lock().map_err(|_| {
        Box::<dyn std::error::Error>::from(std::io::Error::other("fake Go test lock is poisoned"))
    })
}

#[test]
fn resolves_go_modcache_from_root_candidate() {
    let candidate = SelectedCandidate {
        id: None,
        path: PathBuf::from("/Users/me/go/pkg/mod"),
        bytes: 0,
        rule_id: "go.module_cache".to_string(),
        category: Category::Cache,
        safety: Safety::Caution,
        requires_sudo: false,
        risk_score: 0.0,
    };
    assert_eq!(
        go_modcache_path(&candidate),
        Some(PathBuf::from("/Users/me/go/pkg/mod"))
    );
}

#[test]
fn resolves_go_modcache_from_legacy_download_candidate() {
    let candidate = SelectedCandidate {
        id: None,
        path: PathBuf::from("/Users/me/go/pkg/mod/cache/download"),
        bytes: 0,
        rule_id: "go.module_download_cache".to_string(),
        category: Category::Cache,
        safety: Safety::Caution,
        requires_sudo: false,
        risk_score: 0.0,
    };
    assert_eq!(
        go_modcache_path(&candidate),
        Some(PathBuf::from("/Users/me/go/pkg/mod"))
    );
}

#[test]
fn fake_go_modcache_clean_success() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = fake_go_test_guard()?;
    let temp = TempDir::new()?;
    let module_cache = temp.path().join("go").join("pkg").join("mod");
    std::fs::create_dir_all(module_cache.join("cache").join("download"))?;
    let fake_go = write_fake_go_success(&temp.path().join("bin"))?;
    let candidate = go_modcache_candidate(module_cache.clone());

    clean_go_modcache_with_tool(&candidate, fake_go_program(&fake_go)?, FAKE_GO_TEST_TIMEOUT)?;

    let output = std::fs::read_to_string(fake_go_output(&fake_go))?;
    assert!(output.contains("clean"));
    assert!(output.contains("-modcache"));
    assert!(output.contains(&format!("GOMODCACHE={}", module_cache.display())));
    Ok(())
}

#[test]
fn fake_go_modcache_clean_nonzero_is_explicit_failure() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = fake_go_test_guard()?;
    let temp = TempDir::new()?;
    let module_cache = temp.path().join("go").join("pkg").join("mod");
    std::fs::create_dir_all(module_cache.join("cache").join("download"))?;
    let fake_go = write_fake_go_nonzero(&temp.path().join("bin"))?;
    let candidate = go_modcache_candidate(module_cache.clone());

    let err = expected_clean_error(
        clean_go_modcache_with_tool(&candidate, fake_go_program(&fake_go)?, FAKE_GO_TEST_TIMEOUT),
        "nonzero fake go must fail",
    )?;

    assert!(
        err.contains("go clean -modcache failed"),
        "unexpected error: {err}"
    );
    assert!(
        err.contains(&module_cache.display().to_string()),
        "unexpected error: {err}"
    );
    assert!(err.contains("exited"), "unexpected error: {err}");
    assert!(err.contains("permission denied"), "unexpected error: {err}");
    Ok(())
}

#[test]
fn fake_go_modcache_clean_timeout_is_explicit_failure() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = fake_go_test_guard()?;
    let temp = TempDir::new()?;
    let module_cache = temp.path().join("go").join("pkg").join("mod");
    std::fs::create_dir_all(module_cache.join("cache").join("download"))?;
    let fake_go = write_fake_go_timeout(&temp.path().join("bin"))?;
    let candidate = go_modcache_candidate(module_cache.clone());

    let err = expected_clean_error(
        clean_go_modcache_with_tool(
            &candidate,
            fake_go_program(&fake_go)?,
            Duration::from_millis(50),
        ),
        "timed out fake go must fail",
    )?;

    assert!(
        err.contains("go clean -modcache failed"),
        "unexpected error: {err}"
    );
    assert!(
        err.contains(&module_cache.display().to_string()),
        "unexpected error: {err}"
    );
    assert!(err.contains("timed out"), "unexpected error: {err}");
    Ok(())
}

fn go_modcache_candidate(path: PathBuf) -> SelectedCandidate {
    SelectedCandidate {
        id: None,
        path,
        bytes: 0,
        rule_id: "go.module_cache".to_string(),
        category: Category::Cache,
        safety: Safety::Caution,
        requires_sudo: false,
        risk_score: 0.0,
    }
}

fn fake_go_output(fake_go: &Path) -> PathBuf {
    fake_go.with_file_name("out.txt")
}

fn fake_go_program(fake_go: &Path) -> std::io::Result<&str> {
    fake_go.to_str().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "fake go path should be UTF-8",
        )
    })
}

fn expected_clean_error(
    result: Result<(), CleanError>,
    message: &'static str,
) -> Result<String, Box<dyn std::error::Error>> {
    match result {
        Ok(()) => Err(std::io::Error::other(message).into()),
        Err(err) => Ok(err.to_string()),
    }
}

#[cfg(unix)]
fn write_fake_go_success(parent: &Path) -> std::io::Result<PathBuf> {
    write_fake_go(
        parent,
        "#!/bin/sh\nout=\"$(dirname \"$0\")/out.txt\"\nprintf '%s\\n' \"$@\" > \"$out\"\nprintf 'GOMODCACHE=%s\\n' \"$GOMODCACHE\" >> \"$out\"\n",
    )
}

#[cfg(unix)]
fn write_fake_go_nonzero(parent: &Path) -> std::io::Result<PathBuf> {
    write_fake_go(
        parent,
        "#!/bin/sh\nprintf 'permission denied\\n' >&2\nexit 23\n",
    )
}

#[cfg(unix)]
fn write_fake_go_timeout(parent: &Path) -> std::io::Result<PathBuf> {
    write_fake_go(
        parent,
        "#!/bin/sh\nprintf 'started\\n' >&2\nwhile :; do :; done\n",
    )
}

#[cfg(unix)]
fn write_fake_go(parent: &Path, script: &str) -> std::io::Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::create_dir_all(parent)?;
    let fake_go = parent.join("go");
    std::fs::write(&fake_go, script)?;
    let mut permissions = std::fs::metadata(&fake_go)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_go, permissions)?;
    Ok(fake_go)
}

#[cfg(windows)]
fn write_fake_go_success(parent: &Path) -> std::io::Result<PathBuf> {
    write_fake_go(
        parent,
        "@echo off\r\nset OUT=%~dp0out.txt\r\n> \"%OUT%\" (\r\n  echo %1\r\n  echo %2\r\n  echo GOMODCACHE=%GOMODCACHE%\r\n)\r\n",
    )
}

#[cfg(windows)]
fn write_fake_go_nonzero(parent: &Path) -> std::io::Result<PathBuf> {
    write_fake_go(
        parent,
        "@echo off\r\necho permission denied 1>&2\r\nexit /b 23\r\n",
    )
}

#[cfg(windows)]
fn write_fake_go_timeout(parent: &Path) -> std::io::Result<PathBuf> {
    write_fake_go(
        parent,
        "@echo off\r\necho started 1>&2\r\n:loop\r\ngoto loop\r\n",
    )
}

#[cfg(windows)]
fn write_fake_go(parent: &Path, script: &str) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(parent)?;
    let fake_go = parent.join("go.cmd");
    std::fs::write(&fake_go, script)?;
    Ok(fake_go)
}
