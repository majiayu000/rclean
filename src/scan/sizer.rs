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

use rayon::prelude::*;
use tracing::debug;

use crate::model::{CandidateDraft, Safety};

pub(crate) type DirSizes = HashMap<PathBuf, u64>;

pub(crate) struct SizeSummary {
    pub candidate_bytes: Vec<u64>,
    pub source_bytes: u64,
}

pub(crate) fn summarize(
    project_dir: &Path,
    drafts: &[CandidateDraft],
    sizes: &DirSizes,
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
        source_bytes: sum_subtree_bytes(project_dir, sizes),
    }
}

fn dir_size(path: &Path, _verbose: bool) -> u64 {
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
fn sum_subtree_bytes(project_dir: &Path, sizes: &DirSizes) -> u64 {
    let mut total: u64 = 0;
    for (path, bytes) in sizes {
        if path == project_dir || path.starts_with(project_dir) {
            total = total.saturating_add(*bytes);
        }
    }
    total
}
