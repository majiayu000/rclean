//! Parallel scan-phase walker.
//!
//! Phase 1 of `scan::scan()` uses `ignore::WalkParallel` (the same
//! engine ripgrep uses) to visit every entry under each scan root in
//! worker-thread parallelism. The walker:
//!
//!   - Accumulates per-directory file sizes into a `DirSizes` map
//!     (used by `SourceSizeIndex` in phase 2).
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
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use ignore::{WalkBuilder, WalkState};
use tracing::{debug, warn};

use crate::error::ScanError;
use crate::model::{CandidateDraft, ScanWarning};
use crate::path_util::{path_file_name, path_file_name_string};
use crate::rules;
use crate::user_rules::UserRuleSet;

use super::safety::{apply_path_safety, is_skip_dir, is_skip_name};
use super::sizer::DirSizes;
use super::{IgnoreMatcher, ScanOptions, should_include};

/// Thread-safe accumulator shared by every WalkParallel worker.
pub(crate) struct WalkScratch {
    drafts_by_project: Mutex<HashMap<PathBuf, Vec<CandidateDraft>>>,
    sizes: Mutex<DirSizes>,
    warnings: Mutex<Vec<ScanWarning>>,
    poisoned: AtomicBool,
}

impl WalkScratch {
    pub(crate) fn new() -> Self {
        Self {
            drafts_by_project: Mutex::new(HashMap::new()),
            sizes: Mutex::new(HashMap::new()),
            warnings: Mutex::new(Vec::new()),
            poisoned: AtomicBool::new(false),
        }
    }

    pub(crate) fn into_inner(
        self,
    ) -> Result<
        (
            HashMap<PathBuf, Vec<CandidateDraft>>,
            DirSizes,
            Vec<ScanWarning>,
        ),
        ScanError,
    > {
        let WalkScratch {
            drafts_by_project,
            sizes,
            warnings,
            poisoned,
        } = self;
        if poisoned.load(Ordering::SeqCst) {
            return Err(poison_error("walk scratch accumulator"));
        }

        let mut drafts = drafts_by_project
            .into_inner()
            .map_err(|_| poison_error("walk scratch drafts mutex"))?;
        for project_drafts in drafts.values_mut() {
            project_drafts.sort_by(|a, b| a.path.cmp(&b.path));
        }
        let sizes = sizes
            .into_inner()
            .map_err(|_| poison_error("walk scratch sizes mutex"))?;
        let warnings = warnings
            .into_inner()
            .map_err(|_| poison_error("walk scratch warnings mutex"))?;
        Ok((drafts, sizes, warnings))
    }

    fn mark_poisoned(&self, lock_name: &'static str) {
        if !self.poisoned.swap(true, Ordering::SeqCst) {
            warn!(
                lock = lock_name,
                "walk scratch mutex poisoned; aborting scan result"
            );
        }
    }

    fn is_poisoned(&self) -> bool {
        self.poisoned.load(Ordering::SeqCst)
    }
}

struct WalkLocal<'a> {
    scratch: &'a WalkScratch,
    drafts_by_project: HashMap<PathBuf, Vec<CandidateDraft>>,
    sizes: DirSizes,
    warnings: Vec<ScanWarning>,
}

impl<'a> WalkLocal<'a> {
    fn new(scratch: &'a WalkScratch) -> Self {
        Self {
            scratch,
            drafts_by_project: HashMap::new(),
            sizes: HashMap::new(),
            warnings: Vec::new(),
        }
    }

    fn add_file_size(&mut self, parent: &Path, bytes: u64) {
        let entry = self.sizes.entry(parent.to_path_buf()).or_insert(0);
        *entry = entry.saturating_add(bytes);
    }

    fn add_draft(&mut self, project_dir: &Path, draft: CandidateDraft) {
        self.drafts_by_project
            .entry(project_dir.to_path_buf())
            .or_default()
            .push(draft);
    }

    fn add_warning(&mut self, warning: ScanWarning) {
        self.warnings.push(warning);
    }
}

impl Drop for WalkLocal<'_> {
    fn drop(&mut self) {
        let local_sizes = mem::take(&mut self.sizes);
        if !local_sizes.is_empty() {
            match self.scratch.sizes.lock() {
                Ok(mut sizes) => {
                    for (dir, bytes) in local_sizes {
                        let entry = sizes.entry(dir).or_insert(0);
                        *entry = entry.saturating_add(bytes);
                    }
                }
                Err(_) => self.scratch.mark_poisoned("sizes"),
            }
        }

        let local_drafts = mem::take(&mut self.drafts_by_project);
        if !local_drafts.is_empty() {
            match self.scratch.drafts_by_project.lock() {
                Ok(mut drafts) => {
                    for (project_dir, mut project_drafts) in local_drafts {
                        drafts
                            .entry(project_dir)
                            .or_default()
                            .append(&mut project_drafts);
                    }
                }
                Err(_) => self.scratch.mark_poisoned("drafts_by_project"),
            }
        }

        let local_warnings = mem::take(&mut self.warnings);
        if !local_warnings.is_empty() {
            match self.scratch.warnings.lock() {
                Ok(mut warnings) => warnings.extend(local_warnings),
                Err(_) => self.scratch.mark_poisoned("warnings"),
            }
        }
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

    let threads = available_parallelism().map(|n| n.get()).unwrap_or(1).max(1);

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
        let mut local = WalkLocal::new(scratch);
        Box::new(move |result| {
            if scratch.is_poisoned() {
                return WalkState::Quit;
            }

            let entry = match result {
                Ok(e) => e,
                Err(err) => {
                    debug!(error = %err, "walk error");
                    local.add_warning(ScanWarning::WalkError {
                        path: None,
                        error: err.to_string(),
                    });
                    return WalkState::Continue;
                }
            };

            let path = entry.path().to_path_buf();
            let file_type = match entry.file_type() {
                Some(ft) => ft,
                None => return WalkState::Continue,
            };

            // The scan root may itself be a normally skipped toolchain
            // directory, especially with `--home` expansion (`~/.cargo`,
            // `~/.gradle`, `~/Library`, ...). Always allow the explicit
            // root, then apply pruning to descendants.
            if path == root {
                return WalkState::Continue;
            }

            // Pruned subtrees match v0.1.0 is_skip_dir semantics.
            if file_type.is_dir() && (is_skip_dir(&path) || is_skip_name_path(&path)) {
                return WalkState::Skip;
            }

            if file_type.is_file() {
                match entry.metadata() {
                    Ok(metadata) => {
                        if let Some(parent) = path.parent() {
                            local.add_file_size(parent, metadata.len());
                        }
                    }
                    Err(err) => {
                        debug!(path = %path.display(), error = %err, "walk metadata error");
                        local.add_warning(ScanWarning::MetadataError {
                            path,
                            error: err.to_string(),
                        });
                    }
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

            let Some(parent) = path.parent() else {
                return WalkState::Continue;
            };
            let Some(name) = path_file_name_string(&path) else {
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
                    local.add_draft(parent, draft);
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
                    local.add_draft(parent, draft);
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

fn poison_error(lock_name: &str) -> ScanError {
    ScanError::Generic(format!("{lock_name} poisoned during scan"))
}

fn is_skip_name_path(path: &Path) -> bool {
    path_file_name(path).is_some_and(is_skip_name)
}

#[cfg(test)]
mod tests {
    use std::panic::{self, AssertUnwindSafe};

    use super::*;

    #[test]
    fn into_inner_reports_poisoned_scratch_without_panicking() {
        let scratch = WalkScratch::new();
        let poison_result = panic::catch_unwind(AssertUnwindSafe(|| {
            let _guard = match scratch.sizes.lock() {
                Ok(guard) => guard,
                Err(err) => panic!("unexpected pre-existing poison: {err}"),
            };
            panic!("poison sizes");
        }));
        assert!(poison_result.is_err());

        let err = match scratch.into_inner() {
            Ok(_) => panic!("poisoned scratch must error"),
            Err(err) => err,
        };

        assert!(
            err.to_string()
                .contains("walk scratch sizes mutex poisoned")
        );
    }

    #[test]
    fn local_drop_marks_poisoned_scratch_without_panicking() {
        let scratch = WalkScratch::new();
        let poison_result = panic::catch_unwind(AssertUnwindSafe(|| {
            let _guard = match scratch.sizes.lock() {
                Ok(guard) => guard,
                Err(err) => panic!("unexpected pre-existing poison: {err}"),
            };
            panic!("poison sizes");
        }));
        assert!(poison_result.is_err());

        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let mut local = WalkLocal::new(&scratch);
            local.add_file_size(Path::new("/tmp/project"), 7);
        }));

        assert!(result.is_ok());
        assert!(scratch.is_poisoned());
        let err = match scratch.into_inner() {
            Ok(_) => panic!("poisoned scratch must error"),
            Err(err) => err,
        };
        assert!(
            err.to_string()
                .contains("walk scratch accumulator poisoned")
        );
    }

    #[test]
    fn into_inner_reports_poisoned_drafts_without_panicking() {
        let scratch = WalkScratch::new();
        let poison_result = panic::catch_unwind(AssertUnwindSafe(|| {
            let _guard = match scratch.drafts_by_project.lock() {
                Ok(guard) => guard,
                Err(err) => panic!("unexpected pre-existing poison: {err}"),
            };
            panic!("poison drafts");
        }));
        assert!(poison_result.is_err());

        let err = match scratch.into_inner() {
            Ok(_) => panic!("poisoned scratch must error"),
            Err(err) => err,
        };

        assert!(
            err.to_string()
                .contains("walk scratch drafts mutex poisoned")
        );
    }

    #[test]
    fn local_drop_marks_poisoned_drafts_without_panicking() {
        let scratch = WalkScratch::new();
        let poison_result = panic::catch_unwind(AssertUnwindSafe(|| {
            let _guard = match scratch.drafts_by_project.lock() {
                Ok(guard) => guard,
                Err(err) => panic!("unexpected pre-existing poison: {err}"),
            };
            panic!("poison drafts");
        }));
        assert!(poison_result.is_err());

        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let mut local = WalkLocal::new(&scratch);
            let draft = CandidateDraft {
                path: PathBuf::from("/tmp/project/node_modules"),
                name: "node_modules".to_string(),
                rule_id: "node.node_modules".to_string(),
                category: crate::model::Category::Deps,
                safety: crate::model::Safety::Safe,
                reasons: Vec::new(),
                warnings: Vec::new(),
                restore_hint: "reinstall dependencies".to_string(),
            };
            local.add_draft(Path::new("/tmp/project"), draft);
        }));

        assert!(result.is_ok());
        assert!(scratch.is_poisoned());
        let err = match scratch.into_inner() {
            Ok(_) => panic!("poisoned scratch must error"),
            Err(err) => err,
        };
        assert!(
            err.to_string()
                .contains("walk scratch accumulator poisoned")
        );
    }
}
