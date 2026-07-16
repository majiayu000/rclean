//! Git metadata caching for the scan phase.
//!
//! `GitCache` runs `git rev-parse` and `git status --porcelain` at
//! most once per repo per scan. Monorepos with many sibling
//! candidates share one cache entry for the enclosing repo, so we
//! avoid the O(N) `git` subprocess fan-out the v0.1.0 baseline had.
//!
//! The cache is thread-safe so the parallel walker can share one
//! instance across worker threads. Contention is low: the cache
//! transitions only when entering a candidate directory.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{
    RwLock, RwLockReadGuard, RwLockWriteGuard,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use super::DEFAULT_GIT_TIMEOUT;
use crate::model::GitInfo;
use tracing::{info, warn};
use wait_timeout::ChildExt;

pub(crate) struct GitCache {
    by_dir: RwLock<HashMap<PathBuf, GitInfo>>,
    non_repos: RwLock<HashSet<PathBuf>>,
    failed_repos: RwLock<HashSet<PathBuf>>,
    marker_by_dir: RwLock<HashMap<PathBuf, bool>>,
    poisoned: AtomicBool,
    timeout: Option<Duration>,
    discovery_overridden: bool,
    marker_probe: fn(&Path) -> io::Result<fs::Metadata>,
    git_runner: Box<dyn GitRunner>,
}

#[derive(Debug, Eq, PartialEq)]
enum MarkerHint {
    Found,
    Absent,
    Fallback,
}

trait GitRunner: Send + Sync {
    fn rev_parse(&self, dir: &Path, timeout: Duration) -> Option<String>;
    fn dirty(&self, repo_root: &Path, timeout: Duration) -> Option<bool>;
}

struct SystemGitRunner;

impl GitRunner for SystemGitRunner {
    fn rev_parse(&self, dir: &Path, timeout: Duration) -> Option<String> {
        run_git_rev_parse(dir, timeout)
    }

    fn dirty(&self, repo_root: &Path, timeout: Duration) -> Option<bool> {
        run_git_dirty(repo_root, timeout)
    }
}

impl Default for GitCache {
    fn default() -> Self {
        Self::with_timeout(Some(DEFAULT_GIT_TIMEOUT))
    }
}

impl GitCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn with_timeout(timeout: Option<Duration>) -> Self {
        let timeout = timeout.filter(|timeout| !timeout.is_zero());
        if timeout.is_none() {
            info!("git checks disabled by git timeout setting");
        }
        Self::from_parts(
            timeout,
            git_discovery_overridden(),
            probe_marker_metadata,
            Box::new(SystemGitRunner),
        )
    }

    fn from_parts(
        timeout: Option<Duration>,
        discovery_overridden: bool,
        marker_probe: fn(&Path) -> io::Result<fs::Metadata>,
        git_runner: Box<dyn GitRunner>,
    ) -> Self {
        Self {
            by_dir: RwLock::new(HashMap::new()),
            non_repos: RwLock::new(HashSet::new()),
            failed_repos: RwLock::new(HashSet::new()),
            marker_by_dir: RwLock::new(HashMap::new()),
            poisoned: AtomicBool::new(false),
            timeout,
            discovery_overridden,
            marker_probe,
            git_runner,
        }
    }

    pub(crate) fn info_for(&self, dir: &Path) -> Option<GitInfo> {
        let timeout = self.timeout?;
        if !self.is_poisoned() {
            if let Some(info) = self.cached_info(dir) {
                return Some(info);
            }
            if self.is_known_non_repo(dir) {
                return None;
            }
        }

        if self.marker_hint(dir) == MarkerHint::Absent {
            self.remember_non_repo(dir);
            return None;
        }

        let repo_root = match self.git_runner.rev_parse(dir, timeout) {
            Some(root) => root,
            None => {
                self.remember_non_repo(dir);
                return None;
            }
        };

        let root_path = PathBuf::from(&repo_root);
        if self.is_known_failed_repo(&root_path) {
            return None;
        }
        if !self.is_poisoned()
            && let Some(info) = self.cached_info(&root_path)
        {
            self.remember_info(dir, info.clone());
            return Some(info);
        }

        let dirty = match self.git_runner.dirty(&root_path, timeout) {
            Some(dirty) => dirty,
            None => {
                self.remember_failed_repo(&root_path);
                return None;
            }
        };
        let info = GitInfo { repo_root, dirty };
        self.remember_info(&root_path, info.clone());
        self.remember_info(dir, info.clone());
        Some(info)
    }

    fn marker_hint(&self, dir: &Path) -> MarkerHint {
        if self.discovery_overridden || self.is_poisoned() {
            return MarkerHint::Fallback;
        }

        let mut visited = Vec::new();
        let mut current = dir;
        loop {
            match (self.marker_probe)(&current.join(".git")) {
                Ok(_) => {
                    visited.push(current.to_path_buf());
                    return if self.remember_marker_paths(&visited, true) {
                        MarkerHint::Found
                    } else {
                        MarkerHint::Fallback
                    };
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    visited.push(current.to_path_buf());
                }
                Err(_) => return MarkerHint::Fallback,
            }

            let Some(parent) = current.parent() else {
                return if self.remember_marker_paths(&visited, false) {
                    MarkerHint::Absent
                } else {
                    MarkerHint::Fallback
                };
            };

            match self.cached_marker(parent) {
                Ok(Some(found)) => {
                    return if self.remember_marker_paths(&visited, found) {
                        if found {
                            MarkerHint::Found
                        } else {
                            MarkerHint::Absent
                        }
                    } else {
                        MarkerHint::Fallback
                    };
                }
                Ok(None) => current = parent,
                Err(()) => return MarkerHint::Fallback,
            }
        }
    }

    fn cached_marker(&self, dir: &Path) -> Result<Option<bool>, ()> {
        let cache = self
            .read_lock(&self.marker_by_dir, "marker_by_dir")
            .ok_or(())?;
        Ok(cache.get(dir).copied())
    }

    fn remember_marker_paths(&self, paths: &[PathBuf], found: bool) -> bool {
        let Some(mut cache) = self.write_lock(&self.marker_by_dir, "marker_by_dir") else {
            return false;
        };
        for path in paths {
            cache.insert(path.clone(), found);
        }
        true
    }

    fn cached_info(&self, dir: &Path) -> Option<GitInfo> {
        let cache = self.read_lock(&self.by_dir, "by_dir")?;
        cache.get(dir).cloned()
    }

    fn remember_info(&self, dir: &Path, info: GitInfo) {
        if let Some(mut cache) = self.write_lock(&self.by_dir, "by_dir") {
            cache.insert(dir.to_path_buf(), info);
        }
    }

    fn is_known_non_repo(&self, dir: &Path) -> bool {
        self.read_lock(&self.non_repos, "non_repos")
            .is_some_and(|cache| cache.contains(dir))
    }

    fn remember_non_repo(&self, dir: &Path) {
        if let Some(mut cache) = self.write_lock(&self.non_repos, "non_repos") {
            cache.insert(dir.to_path_buf());
        }
    }

    fn is_known_failed_repo(&self, repo_root: &Path) -> bool {
        self.read_lock(&self.failed_repos, "failed_repos")
            .is_some_and(|cache| cache.contains(repo_root))
    }

    fn remember_failed_repo(&self, repo_root: &Path) {
        if let Some(mut cache) = self.write_lock(&self.failed_repos, "failed_repos") {
            cache.insert(repo_root.to_path_buf());
        }
    }

    fn read_lock<'a, T>(
        &self,
        lock: &'a RwLock<T>,
        lock_name: &'static str,
    ) -> Option<RwLockReadGuard<'a, T>> {
        if self.is_poisoned() {
            return None;
        }
        match lock.read() {
            Ok(guard) => Some(guard),
            Err(err) => {
                self.mark_poisoned(lock_name, err);
                None
            }
        }
    }

    fn write_lock<'a, T>(
        &self,
        lock: &'a RwLock<T>,
        lock_name: &'static str,
    ) -> Option<RwLockWriteGuard<'a, T>> {
        if self.is_poisoned() {
            return None;
        }
        match lock.write() {
            Ok(guard) => Some(guard),
            Err(err) => {
                self.mark_poisoned(lock_name, err);
                None
            }
        }
    }

    fn mark_poisoned(&self, lock_name: &'static str, error: impl std::fmt::Display) {
        if !self.poisoned.swap(true, Ordering::SeqCst) {
            warn!(
                lock = lock_name,
                error = %error,
                "git cache lock poisoned; disabling cache for this scan"
            );
        }
    }

    fn is_poisoned(&self) -> bool {
        self.poisoned.load(Ordering::SeqCst)
    }
}

fn git_discovery_overridden() -> bool {
    [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_CEILING_DIRECTORIES",
        "GIT_DISCOVERY_ACROSS_FILESYSTEM",
    ]
    .iter()
    .any(|name| std::env::var_os(name).is_some())
}

fn probe_marker_metadata(path: &Path) -> io::Result<fs::Metadata> {
    fs::symlink_metadata(path)
}

fn run_git_rev_parse(dir: &Path, timeout: Duration) -> Option<String> {
    let output = run_git_command(dir, &["rev-parse", "--show-toplevel"], timeout)?;
    let repo_root = String::from_utf8_lossy(&output).trim().to_string();
    if repo_root.is_empty() {
        None
    } else {
        Some(repo_root)
    }
}

fn run_git_dirty(repo_root: &Path, timeout: Duration) -> Option<bool> {
    run_git_command(repo_root, &["status", "--porcelain", "-uall"], timeout)
        .map(|stdout| !stdout.is_empty())
}

fn run_git_command(dir: &Path, args: &[&str], timeout: Duration) -> Option<Vec<u8>> {
    let mut command = Command::new("git");
    command.arg("-C").arg(dir).args(args);
    command_output_with_timeout(command, timeout, "git", dir, args)
}

fn command_output_with_timeout(
    mut command: Command,
    timeout: Duration,
    program: &'static str,
    dir: &Path,
    args: &[&str],
) -> Option<Vec<u8>> {
    let stdout = tempfile::NamedTempFile::new().ok()?;
    let stdout_writer = stdout.reopen().ok()?;
    let mut child = command
        .stdout(Stdio::from(stdout_writer))
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let status = wait_for_command(&mut child, timeout, program, dir, args)?;
    if !status.success() {
        return None;
    }

    let mut stdout_reader = stdout.reopen().ok()?;
    let mut output = Vec::new();
    stdout_reader.read_to_end(&mut output).ok()?;
    Some(output)
}

fn wait_for_command(
    child: &mut Child,
    timeout: Duration,
    program: &'static str,
    dir: &Path,
    args: &[&str],
) -> Option<ExitStatus> {
    match child.wait_timeout(timeout) {
        Ok(Some(status)) => Some(status),
        Ok(None) => {
            warn!(
                command = program,
                args = ?args,
                dir = %dir.display(),
                timeout_ms = timeout.as_millis(),
                "command timed out; degrading git metadata"
            );
            if let Err(error) = child.kill() {
                warn!(
                    command = program,
                    args = ?args,
                    dir = %dir.display(),
                    error = %error,
                    "failed to kill timed out git command"
                );
            }
            if let Err(error) = child.wait() {
                warn!(
                    command = program,
                    args = ?args,
                    dir = %dir.display(),
                    error = %error,
                    "failed to reap timed out git command"
                );
            }
            None
        }
        Err(error) => {
            warn!(
                command = program,
                args = ?args,
                dir = %dir.display(),
                error = %error,
                "failed while waiting for git command"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests;
