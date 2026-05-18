//! Parallel scan-phase walker.
//!
//! Phase 1 of `scan::scan()` uses `ignore::WalkParallel` (the same
//! engine ripgrep uses) to visit every entry under each scan root in
//! worker-thread parallelism. The walker:
//!
//!   - Accumulates per-directory file sizes into a `DirSizes` map
//!     (used by `sum_subtree_bytes` in phase 2).
//!   - Classifies candidate-named directories and groups the
//!     resulting drafts by project directory.
//!   - Honors the existing `IgnoreMatcher` (`.rcleanignore` + CLI
//!     `--ignore` globs) and the `is_skip_name` / `is_skip_dir`
//!     allowlist.
//!
//! Output is fully deterministic after `scan()` sorts project_dirs by
//! path, so candidate ordering and project ordering don't depend on
//! thread interleaving.
//!
//! This module lives next to `scan.rs` rather than inside it because
//! `scan.rs` is at the U-16 800-line ceiling and adding the parallel
//! walker inline would push it over. Future restructure into
//! `src/scan/{mod,walker,sizer,...}.rs` per SPEC §3.1 lands when M5
//! cleanup hits the file-layout work.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ignore::{WalkBuilder, WalkState};
use tracing::debug;

use crate::model::CandidateDraft;
use crate::rules;
use crate::user_rules::UserRuleSet;

use super::safety::{apply_path_safety, is_skip_dir, is_skip_name};
use super::sizer::DirSizes;
use super::{IgnoreMatcher, ScanOptions, should_include};

/// Thread-safe accumulator shared by every WalkParallel worker.
pub(crate) struct WalkScratch {
    drafts_by_project: Mutex<HashMap<PathBuf, Vec<CandidateDraft>>>,
    sizes: Mutex<DirSizes>,
}

impl WalkScratch {
    pub(crate) fn new() -> Self {
        Self {
            drafts_by_project: Mutex::new(HashMap::new()),
            sizes: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn into_inner(self) -> (HashMap<PathBuf, Vec<CandidateDraft>>, DirSizes) {
        let drafts = self
            .drafts_by_project
            .into_inner()
            .unwrap_or_else(|e| panic!("walk scratch drafts mutex poisoned: {e}"));
        let sizes = self
            .sizes
            .into_inner()
            .unwrap_or_else(|e| panic!("walk scratch sizes mutex poisoned: {e}"));
        (drafts, sizes)
    }
}

pub(crate) fn walk_parallel(
    root: &Path,
    options: &ScanOptions,
    matcher: &IgnoreMatcher,
    user_rules: &UserRuleSet,
    scratch: &WalkScratch,
) {
    use std::thread::available_parallelism;

    let threads = available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .max(1);

    let mut builder = WalkBuilder::new(root);
    builder
        .max_depth(Some(options.max_depth))
        .threads(threads)
        .follow_links(false)
        // rclean targets project artifacts, not source-control
        // opinions — disable .gitignore / .ignore auto-loading.
        // The user's .rcleanignore is layered via IgnoreMatcher
        // before this function runs.
        .standard_filters(false)
        .hidden(false);

    builder.build_parallel().run(|| {
        Box::new(|result| {
            let entry = match result {
                Ok(e) => e,
                Err(err) => {
                    debug!(error = %err, "walk error");
                    return WalkState::Continue;
                }
            };

            let path = entry.path().to_path_buf();
            let file_type = match entry.file_type() {
                Some(ft) => ft,
                None => return WalkState::Continue,
            };

            // Pruned subtrees match v0.1.0 is_skip_dir semantics.
            if file_type.is_dir() && (is_skip_dir(&path) || is_skip_name_path(&path)) {
                return WalkState::Skip;
            }

            if file_type.is_file() {
                if let Ok(metadata) = entry.metadata()
                    && let Some(parent) = path.parent()
                {
                    let mut sizes = scratch.sizes.lock().unwrap_or_else(|e| panic!("walk scratch mutex poisoned: {e}"));
                    let entry = sizes.entry(parent.to_path_buf()).or_insert(0);
                    *entry = entry.saturating_add(metadata.len());
                }
                return WalkState::Continue;
            }

            // Symlinks to directories are not followed (`follow_links(false)`),
            // but the entry itself must still be classified — `apply_path_safety`
            // marks symlinked candidates as Blocked. Treat both dirs and
            // dir-symlinks as classification candidates here.
            let is_dir_like = file_type.is_dir() || file_type.is_symlink();
            if !is_dir_like {
                return WalkState::Continue;
            }

            // Don't reclassify the scan root itself.
            if path == root {
                return WalkState::Continue;
            }

            let Some(parent) = path.parent() else {
                return WalkState::Continue;
            };
            let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
                return WalkState::Continue;
            };

            // Builtin classifier first.
            if rules::is_candidate_name(&name)
                && let Some(mut draft) = rules::classify_candidate(parent, &name, path.clone())
            {
                if matcher.is_ignored(&path, true) {
                    return WalkState::Skip;
                }
                apply_path_safety(root, &mut draft);
                if should_include(&draft, options) {
                    let mut drafts = scratch.drafts_by_project.lock().unwrap_or_else(|e| panic!("walk scratch mutex poisoned: {e}"));
                    drafts.entry(parent.to_path_buf()).or_default().push(draft);
                }
                // Classified candidate's subtree is the candidate's
                // own concern — dir_size walks it separately in
                // phase 2.
                return WalkState::Skip;
            }

            // User rules second.
            if !user_rules.is_empty()
                && let Some(mut draft) = user_rules.classify(&name, parent)
            {
                if matcher.is_ignored(&path, true) {
                    return WalkState::Skip;
                }
                apply_path_safety(root, &mut draft);
                if should_include(&draft, options) {
                    let mut drafts = scratch.drafts_by_project.lock().unwrap_or_else(|e| panic!("walk scratch mutex poisoned: {e}"));
                    drafts.entry(parent.to_path_buf()).or_default().push(draft);
                }
                return WalkState::Skip;
            }

            // Non-candidate dir. Skip if ignored.
            if matcher.is_ignored(&path, true) {
                return WalkState::Skip;
            }
            WalkState::Continue
        })
    });
}

fn is_skip_name_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(is_skip_name)
}
