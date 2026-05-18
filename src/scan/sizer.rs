//! Directory-size accumulation for phase 2 of `scan()`.
//!
//! Phase 1 (`walker`) populates a `DirSizes` map with immediate file
//! byte tallies per directory. Phase 2 folds entries in that map
//! under each project root via [`sum_subtree_bytes`] to get the
//! project's non-artifact source size.
//!
//! Candidate subtrees (e.g. `node_modules`) are absent from the map
//! because the walker prunes them — their bytes come from
//! [`dir_size`] called per-candidate during project materialization.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracing::debug;

pub(crate) type DirSizes = HashMap<PathBuf, u64>;

/// Unbounded subtree byte count. Used for candidate directories
/// whose contents the walker deliberately skipped.
pub(crate) fn dir_size(path: &Path, _verbose: bool) -> u64 {
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

/// Folds every per-directory `file_bytes` tally collected by the
/// walker for paths under `project_dir`. Candidate subtrees are
/// absent from the map (the walker doesn't recurse into them —
/// `dir_size` handles those separately, unbounded), and
/// skipped/excluded names never make it into the map either.
pub(crate) fn sum_subtree_bytes(project_dir: &Path, sizes: &DirSizes) -> u64 {
    let mut total: u64 = 0;
    for (path, bytes) in sizes {
        if path == project_dir || path.starts_with(project_dir) {
            total = total.saturating_add(*bytes);
        }
    }
    total
}
