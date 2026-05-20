use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use tracing::debug;

use crate::model::{CandidateDraft, Safety};

/// Per-directory immediate file-byte tally collected during `scan_dir`.
/// A project's source size is folded from this map instead of doing a
/// dedicated second source-size walk.
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

/// Folds every per-directory `file_bytes` tally that `scan_dir` already
/// collected for paths under `project_dir`. Candidate subtrees are absent
/// from the map because scan traversal does not recurse into them.
fn sum_subtree_bytes(project_dir: &Path, sizes: &DirSizes) -> u64 {
    let mut total: u64 = 0;
    for (path, bytes) in sizes {
        if path == project_dir || path.starts_with(project_dir) {
            total = total.saturating_add(*bytes);
        }
    }
    total
}
