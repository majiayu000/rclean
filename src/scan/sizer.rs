//! Directory-size accumulation for phase 2 of `scan()`.
//!
//! Phase 1 (`walker`) populates a `DirSizes` map with immediate file
//! byte tallies per directory. Phase 2 indexes those tallies once so
//! each project can look up its non-artifact source size directly.
//!
//! Candidate subtrees (e.g. `node_modules`) are absent from the map
//! because the walker prunes them — their bytes come from
//! [`dir_size`] called per-candidate during project materialization.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use tracing::debug;

use crate::model::{CandidateDraft, Safety};

pub(crate) type DirSizes = HashMap<PathBuf, u64>;

pub(crate) struct SourceSizeIndex {
    subtree_bytes: HashMap<PathBuf, u64>,
}

impl SourceSizeIndex {
    pub(crate) fn from_dir_sizes(sizes: &DirSizes) -> Self {
        let mut subtree_bytes: HashMap<PathBuf, u64> = HashMap::new();
        for (dir, bytes) in sizes {
            let mut current = Some(dir.as_path());
            while let Some(path) = current {
                if !path.as_os_str().is_empty() {
                    let entry = subtree_bytes.entry(path.to_path_buf()).or_insert(0);
                    *entry = (*entry).saturating_add(*bytes);
                }
                current = path.parent().filter(|parent| *parent != path);
            }
        }
        Self { subtree_bytes }
    }

    fn bytes_under(&self, project_dir: &Path) -> u64 {
        self.subtree_bytes.get(project_dir).copied().unwrap_or(0)
    }
}

pub(crate) struct SizeSummary {
    pub candidate_bytes: Vec<u64>,
    pub source_bytes: u64,
}

pub(crate) fn summarize(
    project_dir: &Path,
    drafts: &[CandidateDraft],
    source_sizes: &SourceSizeIndex,
    verbose: bool,
) -> SizeSummary {
    let candidate_bytes = drafts
        .par_iter()
        .map(|draft| {
            if draft.safety == Safety::Blocked {
                0
            } else {
                dir_size(&draft.path, verbose)
            }
        })
        .collect();

    SizeSummary {
        candidate_bytes,
        source_bytes: source_sizes.bytes_under(project_dir),
    }
}

fn dir_size(path: &Path, _verbose: bool) -> u64 {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata.len(),
        Ok(metadata) if metadata.is_dir() => {
            let partition = partition_parallel_roots(path);
            partition
                .bytes
                .saturating_add(dir_size_roots(&partition.roots))
        }
        Ok(_) => 0,
        Err(err) => {
            debug!(path = %path.display(), error = %err, "dir_size metadata error");
            0
        }
    }
}

struct SizePartition {
    bytes: u64,
    roots: Vec<PathBuf>,
}

fn partition_parallel_roots(path: &Path) -> SizePartition {
    let mut bytes: u64 = 0;
    let mut current = path.to_path_buf();

    loop {
        let mut subdirs = Vec::new();

        let entries = match fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(err) => {
                debug!(path = %current.display(), error = %err, "dir_size read_dir error");
                return SizePartition {
                    bytes,
                    roots: Vec::new(),
                };
            }
        };

        for result in entries {
            let entry = match result {
                Ok(entry) => entry,
                Err(err) => {
                    debug!(path = %current.display(), error = %err, "dir_size read_dir entry error");
                    continue;
                }
            };
            let entry_path = entry.path();
            match fs::symlink_metadata(&entry_path) {
                Ok(metadata) if metadata.is_file() => {
                    bytes = bytes.saturating_add(metadata.len());
                }
                Ok(metadata) if metadata.is_dir() => {
                    subdirs.push(entry_path);
                }
                Ok(_) => {}
                Err(err) => {
                    debug!(path = %entry_path.display(), error = %err, "dir_size metadata error");
                }
            }
        }

        match subdirs.len() {
            0 => {
                return SizePartition {
                    bytes,
                    roots: Vec::new(),
                };
            }
            1 => {
                current = subdirs
                    .pop()
                    .expect("single-subdir partition should contain one path");
            }
            _ => {
                return SizePartition {
                    bytes,
                    roots: subdirs,
                };
            }
        }
    }
}

fn dir_size_roots(roots: &[PathBuf]) -> u64 {
    match roots {
        [] => 0,
        [only] => dir_size_walkdir(only),
        _ => roots
            .par_iter()
            .map(|path| dir_size_walkdir(path))
            .reduce(|| 0, u64::saturating_add),
    }
}

fn dir_size_walkdir(path: &Path) -> u64 {
    let mut total: u64 = 0;

    for result in walkdir::WalkDir::new(path).follow_links(false) {
        let entry = match result {
            Ok(entry) => entry,
            Err(err) => {
                debug!(path = %path.display(), error = %err, "dir_size walk error");
                continue;
            }
        };
        match entry.metadata() {
            Ok(metadata) if metadata.is_file() => {
                total = total.saturating_add(metadata.len());
            }
            Ok(_) => {}
            Err(err) => {
                debug!(path = %entry.path().display(), error = %err, "dir_size metadata error");
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_size_index_rolls_up_descendants_without_sibling_contamination() {
        let project_a = PathBuf::from("workspace/project_a");
        let project_b = PathBuf::from("workspace/project_b");
        let mut sizes = DirSizes::new();
        sizes.insert(project_a.join("src"), 10);
        sizes.insert(project_a.join("tests"), 5);
        sizes.insert(project_b.join("src"), 7);

        let index = SourceSizeIndex::from_dir_sizes(&sizes);

        assert_eq!(index.bytes_under(&project_a), 15);
        assert_eq!(index.bytes_under(&project_b), 7);
        assert_eq!(index.bytes_under(Path::new("workspace")), 22);
        assert_eq!(index.bytes_under(Path::new("workspace/missing")), 0);
    }
}
