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
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ignore::{WalkBuilder, WalkState};
use rayon::prelude::*;
use tracing::debug;

use crate::model::{CandidateDraft, Safety, ScanWarning};

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
    /// Newest file mtime seen inside each candidate, parallel to
    /// `candidate_bytes`. `None` when the candidate was not walked
    /// (e.g. blocked). Feeds per-candidate `staleness_days` so a cache
    /// under a busy shared parent reports its own age rather than a
    /// sibling's (spec: `specs/GH354/product.md`).
    pub candidate_activity: Vec<Option<SystemTime>>,
    pub source_bytes: u64,
    pub warnings: Vec<ScanWarning>,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct SizeOutcome {
    bytes: u64,
    newest_mtime: Option<SystemTime>,
    warnings: Vec<ScanWarning>,
}

impl SizeOutcome {
    fn with_bytes(bytes: u64) -> Self {
        Self {
            bytes,
            newest_mtime: None,
            warnings: Vec::new(),
        }
    }

    fn merge(&mut self, mut other: Self) {
        self.bytes = self.bytes.saturating_add(other.bytes);
        self.observe_mtime(other.newest_mtime);
        self.warnings.append(&mut other.warnings);
    }

    /// Fold in a file mtime, keeping the newest seen.
    fn observe_mtime(&mut self, mtime: Option<SystemTime>) {
        if let Some(mtime) = mtime
            && self.newest_mtime.is_none_or(|current| mtime > current)
        {
            self.newest_mtime = Some(mtime);
        }
    }

    fn sort_warnings(&mut self) {
        self.warnings
            .sort_by(|left, right| warning_parts(left).cmp(&warning_parts(right)));
    }
}

/// Unix-seconds representation of a mtime for the atomic max in the
/// parallel walk. `0` means "no mtime seen".
fn mtime_to_secs(mtime: SystemTime) -> u64 {
    mtime
        .duration_since(UNIX_EPOCH)
        .map(|age| age.as_secs())
        .unwrap_or(0)
}

fn secs_to_mtime(secs: u64) -> Option<SystemTime> {
    (secs > 0).then(|| UNIX_EPOCH + Duration::from_secs(secs))
}

/// Compare-exchange max, mirroring [`saturating_atomic_add`].
fn atomic_max(slot: &AtomicU64, value: u64) {
    let mut current = slot.load(Ordering::Relaxed);
    while value > current {
        match slot.compare_exchange_weak(current, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(actual) => current = actual,
        }
    }
}

pub(crate) fn summarize(
    project_dir: &Path,
    drafts: &[CandidateDraft],
    source_sizes: &SourceSizeIndex,
    verbose: bool,
) -> SizeSummary {
    let outcomes: Vec<SizeOutcome> = drafts
        .par_iter()
        .map(|draft| {
            if draft.safety == Safety::Blocked {
                SizeOutcome::default()
            } else {
                dir_size(&draft.path, verbose)
            }
        })
        .collect();
    let candidate_bytes = outcomes.iter().map(|outcome| outcome.bytes).collect();
    let candidate_activity = outcomes
        .iter()
        .map(|outcome| outcome.newest_mtime)
        .collect();
    let warnings = outcomes
        .into_iter()
        .flat_map(|outcome| outcome.warnings)
        .collect();

    let source_bytes = if drafts.iter().any(|draft| draft.path == project_dir) {
        0
    } else {
        source_sizes.bytes_under(project_dir)
    };

    SizeSummary {
        candidate_bytes,
        candidate_activity,
        source_bytes,
        warnings,
    }
}

fn dir_size(path: &Path, _verbose: bool) -> SizeOutcome {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => {
            let mut outcome = SizeOutcome::with_bytes(metadata.len());
            outcome.observe_mtime(metadata.modified().ok());
            outcome
        }
        Ok(metadata) if metadata.is_dir() => {
            let mut partition = partition_parallel_roots(path);
            let walked = dir_size_roots(&partition.roots);
            partition.outcome.merge(walked);
            partition.outcome.sort_warnings();
            partition.outcome
        }
        Ok(_) => SizeOutcome::default(),
        Err(err) => {
            debug!(path = %path.display(), error = %err, "dir_size metadata error");
            SizeOutcome {
                bytes: 0,
                newest_mtime: None,
                warnings: vec![ScanWarning::MetadataError {
                    path: path.to_path_buf(),
                    error: err.to_string(),
                }],
            }
        }
    }
}

struct SizePartition {
    outcome: SizeOutcome,
    roots: Vec<PathBuf>,
}

fn partition_parallel_roots(path: &Path) -> SizePartition {
    let mut bytes: u64 = 0;
    let mut newest_mtime: Option<SystemTime> = None;
    let mut current = path.to_path_buf();

    loop {
        if should_walk_parallel(&current) {
            return SizePartition {
                outcome: SizeOutcome {
                    bytes,
                    newest_mtime,
                    warnings: Vec::new(),
                },
                roots: vec![current],
            };
        }

        let mut subdirs = Vec::new();
        let mut warnings = Vec::new();

        let entries = match fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(err) => {
                debug!(path = %current.display(), error = %err, "dir_size read_dir error");
                return SizePartition {
                    outcome: SizeOutcome {
                        bytes,
                        newest_mtime,
                        warnings: vec![ScanWarning::WalkError {
                            path: Some(current),
                            error: err.to_string(),
                        }],
                    },
                    roots: Vec::new(),
                };
            }
        };

        for result in entries {
            let entry = match result {
                Ok(entry) => entry,
                Err(err) => {
                    debug!(path = %current.display(), error = %err, "dir_size read_dir entry error");
                    warnings.push(ScanWarning::WalkError {
                        path: Some(current.clone()),
                        error: err.to_string(),
                    });
                    continue;
                }
            };
            let entry_path = entry.path();
            match fs::symlink_metadata(&entry_path) {
                Ok(metadata) if metadata.is_file() => {
                    bytes = bytes.saturating_add(metadata.len());
                    if let Ok(mtime) = metadata.modified()
                        && newest_mtime.is_none_or(|current| mtime > current)
                    {
                        newest_mtime = Some(mtime);
                    }
                }
                Ok(metadata) if metadata.is_dir() => {
                    subdirs.push(entry_path);
                }
                Ok(_) => {}
                Err(err) => {
                    debug!(path = %entry_path.display(), error = %err, "dir_size metadata error");
                    warnings.push(ScanWarning::MetadataError {
                        path: entry_path,
                        error: err.to_string(),
                    });
                }
            }
        }

        subdirs.sort();

        match subdirs.len() {
            0 => {
                sort_warnings(&mut warnings);
                return SizePartition {
                    outcome: SizeOutcome {
                        bytes,
                        newest_mtime,
                        warnings,
                    },
                    roots: Vec::new(),
                };
            }
            1 => {
                // A directory-entry or metadata error does not make the readable
                // single-child branch disappear. Keep both its bytes and warning.
                // The next iteration cannot carry a separate local vector, so
                // return the child as a root when a warning has already occurred.
                if !warnings.is_empty() {
                    sort_warnings(&mut warnings);
                    return SizePartition {
                        outcome: SizeOutcome {
                            bytes,
                            newest_mtime,
                            warnings,
                        },
                        roots: subdirs,
                    };
                }
                current = subdirs
                    .pop()
                    .expect("single-subdir partition should contain one path");
            }
            _ => {
                sort_warnings(&mut warnings);
                return SizePartition {
                    outcome: SizeOutcome {
                        bytes,
                        newest_mtime,
                        warnings,
                    },
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

fn dir_size_roots(roots: &[PathBuf]) -> SizeOutcome {
    match roots {
        [] => SizeOutcome::default(),
        [only] => dir_size_walk_parallel(only),
        _ => {
            let outcomes: Vec<SizeOutcome> = roots
                .par_iter()
                .map(|path| dir_size_walkdir(path))
                .collect();
            let mut combined = SizeOutcome::default();
            for outcome in outcomes {
                combined.merge(outcome);
            }
            combined.sort_warnings();
            combined
        }
    }
}

fn dir_size_walk_parallel(path: &Path) -> SizeOutcome {
    let total = Arc::new(AtomicU64::new(0));
    // Newest file mtime as Unix seconds; 0 means none seen.
    let newest_secs = Arc::new(AtomicU64::new(0));
    let warnings = Arc::new(Mutex::new(Vec::new()));
    let walk_root = path.to_path_buf();

    let mut builder = WalkBuilder::new(path);
    builder
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .threads(dir_size_threads());

    builder.build_parallel().run(|| {
        let total = Arc::clone(&total);
        let newest_secs = Arc::clone(&newest_secs);
        let warnings = Arc::clone(&warnings);
        let walk_root = walk_root.clone();
        Box::new(move |result| {
            let entry = match result {
                Ok(entry) => entry,
                Err(err) => {
                    debug!(path = %walk_root.display(), error = %err, "dir_size walk error");
                    push_ignore_walk_warnings(&warnings, &err, &walk_root);
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
                    saturating_atomic_add(&total, metadata.len());
                    if let Ok(mtime) = metadata.modified() {
                        atomic_max(&newest_secs, mtime_to_secs(mtime));
                    }
                }
                Err(err) => {
                    debug!(path = %entry.path().display(), error = %err, "dir_size metadata error");
                    push_parallel_warning(
                        &warnings,
                        ScanWarning::MetadataError {
                            path: entry.path().to_path_buf(),
                            error: err.to_string(),
                        },
                    );
                }
            }
            WalkState::Continue
        })
    });

    let mut warnings = match warnings.lock() {
        Ok(warnings) => warnings.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
    sort_warnings(&mut warnings);
    SizeOutcome {
        bytes: total.load(Ordering::Relaxed),
        newest_mtime: secs_to_mtime(newest_secs.load(Ordering::Relaxed)),
        warnings,
    }
}

fn push_parallel_warning(warnings: &Mutex<Vec<ScanWarning>>, warning: ScanWarning) {
    match warnings.lock() {
        Ok(mut warnings) => warnings.push(warning),
        Err(poisoned) => poisoned.into_inner().push(warning),
    }
}

fn push_ignore_walk_warnings(
    warnings: &Mutex<Vec<ScanWarning>>,
    error: &ignore::Error,
    fallback_path: &Path,
) {
    if let ignore::Error::Partial(errors) = error {
        for error in errors {
            push_ignore_walk_warnings(warnings, error, fallback_path);
        }
        return;
    }

    push_parallel_warning(
        warnings,
        ScanWarning::WalkError {
            path: ignore_error_path(error).or_else(|| Some(fallback_path.to_path_buf())),
            error: error.to_string(),
        },
    );
}

fn ignore_error_path(error: &ignore::Error) -> Option<PathBuf> {
    match error {
        ignore::Error::Partial(errors) => errors.iter().find_map(ignore_error_path),
        ignore::Error::WithLineNumber { err, .. } | ignore::Error::WithDepth { err, .. } => {
            ignore_error_path(err)
        }
        ignore::Error::WithPath { path, .. } => Some(path.clone()),
        ignore::Error::Loop { child, .. } => Some(child.clone()),
        ignore::Error::Io(_)
        | ignore::Error::Glob { .. }
        | ignore::Error::UnrecognizedFileType(_)
        | ignore::Error::InvalidDefinition => None,
    }
}

fn saturating_atomic_add(total: &AtomicU64, bytes: u64) {
    let mut current = total.load(Ordering::Relaxed);
    loop {
        let next = current.saturating_add(bytes);
        match total.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(actual) => current = actual,
        }
    }
}

fn dir_size_threads() -> usize {
    std::thread::available_parallelism()
        .map(|threads| threads.get())
        .unwrap_or(1)
        .clamp(1, MAX_DIR_SIZE_THREADS)
}

fn dir_size_walkdir(path: &Path) -> SizeOutcome {
    let mut outcome = SizeOutcome::default();

    for result in walkdir::WalkDir::new(path).follow_links(false) {
        let entry = match result {
            Ok(entry) => entry,
            Err(err) => {
                debug!(path = %path.display(), error = %err, "dir_size walk error");
                outcome.warnings.push(ScanWarning::WalkError {
                    path: err
                        .path()
                        .map(Path::to_path_buf)
                        .or_else(|| Some(path.to_path_buf())),
                    error: err.to_string(),
                });
                continue;
            }
        };
        match entry.metadata() {
            Ok(metadata) if metadata.is_file() => {
                outcome.bytes = outcome.bytes.saturating_add(metadata.len());
                outcome.observe_mtime(metadata.modified().ok());
            }
            Ok(_) => {}
            Err(err) => {
                debug!(path = %entry.path().display(), error = %err, "dir_size metadata error");
                outcome.warnings.push(ScanWarning::MetadataError {
                    path: entry.path().to_path_buf(),
                    error: err.to_string(),
                });
            }
        }
    }
    outcome.sort_warnings();
    outcome
}

fn sort_warnings(warnings: &mut [ScanWarning]) {
    warnings.sort_by(|left, right| warning_parts(left).cmp(&warning_parts(right)));
}

fn warning_parts(warning: &ScanWarning) -> (u8, Option<&Path>, &str) {
    match warning {
        ScanWarning::IgnoreFileLoad { path, error } => (0, Some(path), error),
        ScanWarning::MetadataError { path, error } => (1, Some(path), error),
        ScanWarning::WalkError { path, error } => (2, path.as_deref(), error),
    }
}

#[cfg(test)]
mod tests;
