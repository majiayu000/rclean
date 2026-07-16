use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, atomic::AtomicUsize};
use std::time::Duration;

use tempfile::TempDir;

use super::*;

#[derive(Default)]
struct FakeGitState {
    rev_parse_calls: AtomicUsize,
}

struct FailingGitRunner {
    state: Arc<FakeGitState>,
}

impl GitRunner for FailingGitRunner {
    fn rev_parse(&self, _dir: &Path, _timeout: Duration) -> Option<String> {
        self.state
            .rev_parse_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        None
    }

    fn dirty(&self, _repo_root: &Path, _timeout: Duration) -> Option<bool> {
        panic!("dirty must not run after failed rev-parse")
    }
}

fn test_cache(
    state: Arc<FakeGitState>,
    marker_probe: fn(&Path) -> io::Result<fs::Metadata>,
    discovery_overridden: bool,
) -> GitCache {
    GitCache::from_parts(
        Some(Duration::from_secs(1)),
        discovery_overridden,
        marker_probe,
        Box::new(FailingGitRunner { state }),
    )
}

fn rev_parse_calls(state: &FakeGitState) -> usize {
    state
        .rev_parse_calls
        .load(std::sync::atomic::Ordering::SeqCst)
}

fn marker_permission_denied(_path: &Path) -> io::Result<fs::Metadata> {
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "injected marker metadata error",
    ))
}

fn init_repo(path: &Path) -> io::Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("init")
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other("git init failed"))
    }
}

#[test]
fn no_marker_siblings_skip_git_discovery() -> io::Result<()> {
    let temp = TempDir::new()?;
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    fs::create_dir_all(&first)?;
    fs::create_dir_all(&second)?;
    let state = Arc::new(FakeGitState::default());
    let cache = test_cache(Arc::clone(&state), probe_marker_metadata, false);

    assert!(cache.info_for(&first).is_none());
    assert!(cache.info_for(&second).is_none());
    assert_eq!(rev_parse_calls(&state), 0);
    Ok(())
}

#[test]
fn child_file_and_directory_markers_beat_parent_absence() -> io::Result<()> {
    for marker_is_dir in [false, true] {
        let temp = TempDir::new()?;
        let first = temp.path().join("first");
        let child = temp.path().join("child");
        fs::create_dir_all(&first)?;
        fs::create_dir_all(&child)?;
        let state = Arc::new(FakeGitState::default());
        let cache = test_cache(Arc::clone(&state), probe_marker_metadata, false);
        assert!(cache.info_for(&first).is_none());

        if marker_is_dir {
            fs::create_dir(child.join(".git"))?;
        } else {
            fs::write(child.join(".git"), "gitdir: elsewhere")?;
        }
        assert!(cache.info_for(&child).is_none());
        assert_eq!(rev_parse_calls(&state), 1);
    }
    Ok(())
}

#[test]
fn discovery_override_falls_back_to_git() -> io::Result<()> {
    let temp = TempDir::new()?;
    let state = Arc::new(FakeGitState::default());
    let cache = test_cache(Arc::clone(&state), probe_marker_metadata, true);

    assert!(cache.info_for(temp.path()).is_none());
    assert_eq!(rev_parse_calls(&state), 1);
    Ok(())
}

#[test]
fn marker_metadata_error_falls_back_to_git() -> io::Result<()> {
    let temp = TempDir::new()?;
    let state = Arc::new(FakeGitState::default());
    let cache = test_cache(Arc::clone(&state), marker_permission_denied, false);

    assert!(cache.info_for(temp.path()).is_none());
    assert_eq!(rev_parse_calls(&state), 1);
    Ok(())
}

#[test]
fn poisoned_marker_cache_falls_back_to_git() -> io::Result<()> {
    let temp = TempDir::new()?;
    let state = Arc::new(FakeGitState::default());
    let cache = test_cache(Arc::clone(&state), probe_marker_metadata, false);
    let poison_result = panic::catch_unwind(AssertUnwindSafe(|| {
        let _guard = cache
            .marker_by_dir
            .write()
            .unwrap_or_else(|error| panic!("unexpected pre-existing poison: {error}"));
        panic!("poison marker_by_dir");
    }));
    assert!(poison_result.is_err());

    assert!(cache.info_for(temp.path()).is_none());
    assert_eq!(rev_parse_calls(&state), 1);
    assert!(cache.is_poisoned());
    Ok(())
}

#[test]
fn marker_cache_is_scan_local() -> io::Result<()> {
    let temp = TempDir::new()?;
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    fs::create_dir_all(&first)?;
    fs::create_dir_all(&second)?;
    let first_state = Arc::new(FakeGitState::default());
    let first_cache = test_cache(Arc::clone(&first_state), probe_marker_metadata, false);
    assert!(first_cache.info_for(&first).is_none());

    fs::create_dir(temp.path().join(".git"))?;
    assert!(first_cache.info_for(&second).is_none());
    assert_eq!(rev_parse_calls(&first_state), 0);

    let second_state = Arc::new(FakeGitState::default());
    let second_cache = test_cache(Arc::clone(&second_state), probe_marker_metadata, false);
    assert!(second_cache.info_for(&second).is_none());
    assert_eq!(rev_parse_calls(&second_state), 1);
    Ok(())
}

#[test]
fn parent_repo_marker_uses_git_for_root() -> io::Result<()> {
    let temp = TempDir::new()?;
    init_repo(temp.path())?;
    let child = temp.path().join("nested/project");
    fs::create_dir_all(&child)?;

    let info = GitCache::new()
        .info_for(&child)
        .unwrap_or_else(|| panic!("parent repository should be discovered"));
    assert_eq!(
        PathBuf::from(info.repo_root).canonicalize()?,
        temp.path().canonicalize()?
    );
    Ok(())
}

#[test]
fn nested_repo_marker_beats_cached_parent_hint() -> io::Result<()> {
    let temp = TempDir::new()?;
    init_repo(temp.path())?;
    let sibling = temp.path().join("sibling");
    let nested = temp.path().join("nested");
    fs::create_dir_all(&sibling)?;
    fs::create_dir_all(&nested)?;
    let cache = GitCache::new();
    assert!(cache.info_for(&sibling).is_some());

    init_repo(&nested)?;
    fs::write(nested.join("dirty.txt"), "dirty")?;
    let info = cache
        .info_for(&nested)
        .unwrap_or_else(|| panic!("nested repository should be discovered"));
    assert_eq!(
        PathBuf::from(info.repo_root).canonicalize()?,
        nested.canonicalize()?
    );
    assert!(info.dirty);
    Ok(())
}

#[test]
fn poisoned_by_dir_cache_recomputes_git_info() -> std::io::Result<()> {
    let temp = TempDir::new()?;
    let init = Command::new("git")
        .arg("-C")
        .arg(temp.path())
        .arg("init")
        .output()?;
    assert!(init.status.success());

    let cache = GitCache::new();
    let stale = GitInfo {
        repo_root: "stale".to_string(),
        dirty: false,
    };
    let cache_key = temp.path().to_path_buf();
    let poison_result = panic::catch_unwind(AssertUnwindSafe(|| {
        let mut by_dir = match cache.by_dir.write() {
            Ok(guard) => guard,
            Err(err) => panic!("unexpected pre-existing poison: {err}"),
        };
        by_dir.insert(cache_key, stale);
        panic!("poison by_dir");
    }));
    assert!(poison_result.is_err());

    fs::write(temp.path().join("dirty.txt"), "x")?;
    let info = match cache.info_for(temp.path()) {
        Some(info) => info,
        None => panic!("fresh git info should be available"),
    };

    assert_ne!(info.repo_root, "stale");
    assert!(info.dirty);
    assert!(cache.is_poisoned());
    Ok(())
}

#[test]
fn poisoned_non_repos_cache_recomputes_git_info() -> std::io::Result<()> {
    let temp = TempDir::new()?;
    let init = Command::new("git")
        .arg("-C")
        .arg(temp.path())
        .arg("init")
        .output()?;
    assert!(init.status.success());

    let cache = GitCache::new();
    let cache_key = temp.path().to_path_buf();
    let poison_result = panic::catch_unwind(AssertUnwindSafe(|| {
        let mut non_repos = match cache.non_repos.write() {
            Ok(guard) => guard,
            Err(err) => panic!("unexpected pre-existing poison: {err}"),
        };
        non_repos.insert(cache_key);
        panic!("poison non_repos");
    }));
    assert!(poison_result.is_err());

    let info = match cache.info_for(temp.path()) {
        Some(info) => info,
        None => panic!("fresh git info should be available"),
    };

    assert_ne!(info.repo_root, "");
    assert!(cache.is_poisoned());
    Ok(())
}

#[test]
fn disabled_git_cache_returns_no_info() -> std::io::Result<()> {
    let temp = TempDir::new()?;
    let init = Command::new("git")
        .arg("-C")
        .arg(temp.path())
        .arg("init")
        .output()?;
    assert!(init.status.success());

    let cache = GitCache::with_timeout(None);

    assert!(cache.info_for(temp.path()).is_none());
    Ok(())
}

#[test]
fn command_timeout_returns_none() {
    let output = command_output_with_timeout(
        slow_command(),
        Duration::from_millis(50),
        "test-command",
        Path::new("."),
        &["sleep"],
    );

    assert!(output.is_none());
}

#[cfg(unix)]
fn slow_command() -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", "sleep 1"]);
    command
}

#[cfg(windows)]
fn slow_command() -> Command {
    let mut command = Command::new("cmd");
    command.args(["/C", "ping -n 2 127.0.0.1 >NUL"]);
    command
}
