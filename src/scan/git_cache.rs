//! Git metadata caching for the scan phase.
//!
//! `GitCache` runs `git rev-parse` and `git status --porcelain` at
//! most once per repo per scan. Monorepos with many sibling
//! candidates share one cache entry for the enclosing repo, so we
//! avoid the O(N) `git` subprocess fan-out the v0.1.0 baseline had.
//!
//! `Mutex<HashMap>` (not `RefCell`) so the parallel walker can share
//! one cache across worker threads. Contention is low: the cache
//! transitions only when entering a candidate directory.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use crate::model::GitInfo;

#[derive(Default)]
pub(crate) struct GitCache {
    by_dir: Mutex<HashMap<PathBuf, Option<GitInfo>>>,
}

impl GitCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn info_for(&self, dir: &Path) -> Option<GitInfo> {
        // Phase 1: cache lookup. Scope the guard so it drops before we
        // run git subprocesses below — otherwise the guard's lifetime
        // can extend past the `if let` and deadlock on the next lock().
        {
            let map = self
                .by_dir
                .lock()
                .unwrap_or_else(|e| panic!("git cache mutex poisoned: {e}"));
            if let Some(cached) = map.get(dir) {
                return cached.clone();
            }
        }

        let repo_root = match run_git_rev_parse(dir) {
            Some(root) => root,
            None => {
                self.by_dir
                    .lock()
                    .unwrap_or_else(|e| panic!("git cache mutex poisoned: {e}"))
                    .insert(dir.to_path_buf(), None);
                return None;
            }
        };

        let root_path = PathBuf::from(&repo_root);
        let cached_root = {
            let map = self
                .by_dir
                .lock()
                .unwrap_or_else(|e| panic!("git cache mutex poisoned: {e}"));
            map.get(&root_path).cloned()
        };
        if let Some(Some(info)) = cached_root {
            self.by_dir
                .lock()
                .unwrap_or_else(|e| panic!("git cache mutex poisoned: {e}"))
                .insert(dir.to_path_buf(), Some(info.clone()));
            return Some(info);
        }

        let dirty = run_git_dirty(&root_path);
        let info = GitInfo { repo_root, dirty };
        let mut map = self
            .by_dir
            .lock()
            .unwrap_or_else(|e| panic!("git cache mutex poisoned: {e}"));
        map.insert(root_path, Some(info.clone()));
        map.insert(dir.to_path_buf(), Some(info.clone()));
        Some(info)
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
        .args(["status", "--porcelain"])
        .output();
    matches!(output, Ok(o) if o.status.success() && !o.stdout.is_empty())
}
