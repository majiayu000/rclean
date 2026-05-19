use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::model::GitInfo;

#[derive(Default)]
pub(crate) struct GitCache {
    by_dir: RwLock<HashMap<PathBuf, GitInfo>>,
    non_repos: RwLock<HashSet<PathBuf>>,
}

impl GitCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn info_for(&self, dir: &Path) -> Option<GitInfo> {
        if let Some(info) = self.cached_info(dir) {
            return Some(info);
        }
        if self.is_known_non_repo(dir) {
            return None;
        }

        let repo_root = match run_git_rev_parse(dir) {
            Some(root) => root,
            None => {
                self.remember_non_repo(dir);
                return None;
            }
        };

        let root_path = PathBuf::from(&repo_root);
        if let Some(info) = self.cached_info(&root_path) {
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
        read_lock(&self.by_dir).get(dir).cloned()
    }

    fn remember_info(&self, dir: &Path, info: GitInfo) {
        write_lock(&self.by_dir).insert(dir.to_path_buf(), info);
    }

    fn is_known_non_repo(&self, dir: &Path) -> bool {
        read_lock(&self.non_repos).contains(dir)
    }

    fn remember_non_repo(&self, dir: &Path) {
        write_lock(&self.non_repos).insert(dir.to_path_buf());
    }
}

fn read_lock<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write_lock<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
