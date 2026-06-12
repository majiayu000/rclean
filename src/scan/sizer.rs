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
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use ignore::{WalkBuilder, WalkState};
use rayon::prelude::*;
use tracing::debug;

use crate::model::{CandidateDraft, Safety};

pub(crate) type DirSizes = HashMap<PathBuf, u64>;
const PARALLEL_DIRECT_ENTRY_THRESHOLD: usize = 1_000;
const MAX_DIR_SIZE_THREADS: usize = 4;

pub(crate) struct SourceSizeIndex {
    subtree_bytes: HashMap<PathBuf, u64>,
}

impl SourceSizeIndex {
    pub(crate) fn from_dir_sizes(sizes: &DirSizes) -> Self {
        let mut subtree_bytes: HashMap<PathBuf, u64> = sizes
            .iter()
            .filter(|(dir, _)| !dir.as_os_str().is_empty())
            .map(|(dir, bytes)| (dir.clone(), *bytes))
            .collect();

        for dir in sizes.keys().filter(|dir| !dir.as_os_str().is_empty()) {
            let mut current = indexed_parent(dir);
            while let Some(path) = current {
                if subtree_bytes.contains_key(path) {
                    break;
                }
                subtree_bytes.insert(path.to_path_buf(), 0);
                current = indexed_parent(path);
            }
        }

        let mut dirs: Vec<(usize, PathBuf)> = subtree_bytes
            .keys()
            .map(|dir| (dir.components().count(), dir.clone()))
            .collect();
        dirs.sort_by(|(a_depth, a), (b_depth, b)| b_depth.cmp(a_depth).then_with(|| b.cmp(a)));

        for (_, dir) in dirs {
            let bytes = subtree_bytes.get(&dir).copied().unwrap_or(0);
            if bytes == 0 {
                continue;
            }
            if let Some(parent) = indexed_parent(&dir) {
                let Some(entry) = subtree_bytes.get_mut(parent) else {
                    continue;
                };
                *entry = (*entry).saturating_add(bytes);
            }
        }
        Self { subtree_bytes }
    }

    fn bytes_under(&self, project_dir: &Path) -> u64 {
        self.subtree_bytes.get(project_dir).copied().unwrap_or(0)
    }
}

fn indexed_parent(path: &Path) -> Option<&Path> {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty() && *parent != path)
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
        if should_walk_parallel(&current) {
            return SizePartition {
                bytes,
                roots: vec![current],
            };
        }

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

fn should_walk_parallel(path: &Path) -> bool {
    let Ok(entries) = fs::read_dir(path) else {
        return false;
    };
    entries.take(PARALLEL_DIRECT_ENTRY_THRESHOLD + 1).count() > PARALLEL_DIRECT_ENTRY_THRESHOLD
}

fn dir_size_roots(roots: &[PathBuf]) -> u64 {
    match roots {
        [] => 0,
        [only] => dir_size_walk_parallel(only),
        _ => roots
            .par_iter()
            .map(|path| dir_size_walkdir(path))
            .reduce(|| 0, u64::saturating_add),
    }
}

fn dir_size_walk_parallel(path: &Path) -> u64 {
    let total = Arc::new(AtomicU64::new(0));
    let walk_root = path.to_path_buf();

    let mut builder = WalkBuilder::new(path);
    builder
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .threads(dir_size_threads());

    builder.build_parallel().run(|| {
        let total = Arc::clone(&total);
        let walk_root = walk_root.clone();
        Box::new(move |result| {
            let entry = match result {
                Ok(entry) => entry,
                Err(err) => {
                    debug!(path = %walk_root.display(), error = %err, "dir_size walk error");
                    return WalkState::Continue;
                }
            };

            if !entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
            {
                return WalkState::Continue;
            }

            match entry.metadata() {
                Ok(metadata) => {
                    total.fetch_add(metadata.len(), Ordering::Relaxed);
                }
                Err(err) => {
                    debug!(path = %entry.path().display(), error = %err, "dir_size metadata error");
                }
            }
            WalkState::Continue
        })
    });

    total.load(Ordering::Relaxed)
}

fn dir_size_threads() -> usize {
    std::thread::available_parallelism()
        .map(|threads| threads.get())
        .unwrap_or(1)
        .clamp(1, MAX_DIR_SIZE_THREADS)
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
    use tempfile::TempDir;

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

    #[test]
    fn source_size_index_rolls_up_nested_and_sibling_dirs() {
        let project = PathBuf::from("workspace/app");
        let sibling = PathBuf::from("workspace/other");
        let mut sizes = DirSizes::new();
        sizes.insert(project.join("src"), 2);
        sizes.insert(project.join("src/routes"), 3);
        sizes.insert(project.join("src/routes/api"), 5);
        sizes.insert(project.join("assets/images"), 7);
        sizes.insert(sibling.join("src"), 11);

        let index = SourceSizeIndex::from_dir_sizes(&sizes);

        assert_eq!(index.bytes_under(&project), 17);
        assert_eq!(index.bytes_under(&project.join("src")), 10);
        assert_eq!(index.bytes_under(&project.join("src/routes")), 8);
        assert_eq!(index.bytes_under(&project.join("assets")), 7);
        assert_eq!(index.bytes_under(&sibling), 11);
        assert_eq!(index.bytes_under(Path::new("workspace")), 28);
        assert_eq!(index.bytes_under(Path::new("workspace/app/src/views")), 0);
    }

    #[test]
    fn parallel_walk_matches_serial_walk_for_nested_tree() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("root.bin"), [0; 3]).unwrap();
        fs::create_dir_all(temp.path().join("a/b")).unwrap();
        fs::write(temp.path().join("a/b/leaf.bin"), [0; 5]).unwrap();
        fs::create_dir(temp.path().join("c")).unwrap();
        fs::write(temp.path().join("c/leaf.bin"), [0; 7]).unwrap();

        assert_eq!(
            dir_size_walk_parallel(temp.path()),
            dir_size_walkdir(temp.path())
        );
    }

    #[test]
    fn wide_directory_uses_parallel_root_and_counts_all_files() {
        let temp = TempDir::new().unwrap();
        for i in 0..=PARALLEL_DIRECT_ENTRY_THRESHOLD {
            fs::write(temp.path().join(format!("file_{i:04}.bin")), [0; 2]).unwrap();
        }

        let partition = partition_parallel_roots(temp.path());

        assert_eq!(partition.bytes, 0);
        assert_eq!(partition.roots, vec![temp.path().to_path_buf()]);
        assert_eq!(
            dir_size(temp.path(), false),
            ((PARALLEL_DIRECT_ENTRY_THRESHOLD + 1) * 2) as u64
        );
    }
}
