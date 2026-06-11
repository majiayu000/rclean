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
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    RwLock, RwLockReadGuard, RwLockWriteGuard,
    atomic::{AtomicBool, Ordering},
};

use crate::model::GitInfo;
use tracing::warn;

#[derive(Default)]
pub(crate) struct GitCache {
    by_dir: RwLock<HashMap<PathBuf, GitInfo>>,
    non_repos: RwLock<HashSet<PathBuf>>,
    poisoned: AtomicBool,
}

impl GitCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn info_for(&self, dir: &Path) -> Option<GitInfo> {
        if !self.is_poisoned() {
            if let Some(info) = self.cached_info(dir) {
                return Some(info);
            }
            if self.is_known_non_repo(dir) {
                return None;
            }
        }

        let repo_root = match run_git_rev_parse(dir) {
            Some(root) => root,
            None => {
                self.remember_non_repo(dir);
                return None;
            }
        };

        let root_path = PathBuf::from(&repo_root);
        if !self.is_poisoned()
            && let Some(info) = self.cached_info(&root_path)
        {
            self.remember_info(dir, info.clone());
            return Some(info);
        }

        let info = GitInfo {
            repo_root,
            dirty: run_git_dirty(&root_path),
        };
        self.remember_info(&root_path, info.clone());
        self.remember_info(dir, info.clone());
        Some(info)
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

fn run_git_rev_parse(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let repo_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if repo_root.is_empty() {
        None
    } else {
        Some(repo_root)
    }
}

fn run_git_dirty(repo_root: &Path) -> bool {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["status", "--porcelain", "-uall"])
        .output();
    matches!(output, Ok(o) if o.status.success() && !o.stdout.is_empty())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::panic::{self, AssertUnwindSafe};

    use tempfile::TempDir;

    use super::*;

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
}
