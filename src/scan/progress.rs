//! Streaming scan progress (spec: docs/specs/v0.2-best-ux.md §3.4 D2).
//!
//! A background thread prints a single self-overwriting line to stderr
//! once a scan has been running for about a second, so long scans show
//! movement without touching stdout. stdout stays byte-identical —
//! `--json` consumers never see progress bytes.

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

/// Shared counters bumped by the walker and the project loop.
#[derive(Default)]
pub(crate) struct ProgressCounters {
    projects: AtomicUsize,
    candidates: AtomicUsize,
}

impl ProgressCounters {
    pub(crate) fn add_project(&self) {
        self.projects.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn add_candidates(&self, count: usize) {
        self.candidates.fetch_add(count, Ordering::Relaxed);
    }
}

pub(crate) struct ProgressReporter {
    counters: Arc<ProgressCounters>,
    done: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

/// Delay before the first progress line: fast scans finish silently.
const INITIAL_DELAY: Duration = Duration::from_millis(1000);
const TICK: Duration = Duration::from_millis(200);

impl ProgressReporter {
    pub(crate) fn start() -> Self {
        let counters = Arc::new(ProgressCounters::default());
        let done = Arc::new(AtomicBool::new(false));
        let thread_counters = Arc::clone(&counters);
        let thread_done = Arc::clone(&done);
        let handle = std::thread::spawn(move || {
            let mut waited = Duration::ZERO;
            while waited < INITIAL_DELAY {
                if thread_done.load(Ordering::Relaxed) {
                    return;
                }
                std::thread::sleep(TICK);
                waited += TICK;
            }
            let mut printed = false;
            while !thread_done.load(Ordering::Relaxed) {
                let line = progress_line(
                    thread_counters.projects.load(Ordering::Relaxed),
                    thread_counters.candidates.load(Ordering::Relaxed),
                );
                // \r + erase-to-end keeps it a single updating line.
                let _ = write!(std::io::stderr(), "\r{line}\u{1b}[K");
                let _ = std::io::stderr().flush();
                printed = true;
                std::thread::sleep(TICK);
            }
            if printed {
                let _ = write!(std::io::stderr(), "\r\u{1b}[K");
                let _ = std::io::stderr().flush();
            }
        });
        Self {
            counters,
            done,
            handle: Some(handle),
        }
    }

    pub(crate) fn counters(&self) -> Arc<ProgressCounters> {
        Arc::clone(&self.counters)
    }

    pub(crate) fn finish(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        self.done.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for ProgressReporter {
    fn drop(&mut self) {
        self.stop();
    }
}

fn progress_line(projects: usize, candidates: usize) -> String {
    format!("scanning: {projects} project(s), {candidates} candidate(s) so far")
}

/// Whether to stream progress for this invocation. `RCLEAN_PROGRESS`
/// (`always` / `never`) overrides the default: an interactive stderr,
/// and never under `--json` unless `--verbose` asks for diagnostics.
pub(crate) fn progress_enabled(json: bool, verbose: bool) -> bool {
    use std::io::IsTerminal;
    match std::env::var("RCLEAN_PROGRESS").as_deref() {
        Ok("always") => true,
        Ok("never") => false,
        _ => std::io::stderr().is_terminal() && (!json || verbose),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_line_reports_both_counters() {
        assert_eq!(
            progress_line(3, 17),
            "scanning: 3 project(s), 17 candidate(s) so far"
        );
    }

    #[test]
    fn counters_accumulate() {
        let counters = ProgressCounters::default();
        counters.add_project();
        counters.add_project();
        counters.add_candidates(5);
        assert_eq!(counters.projects.load(Ordering::Relaxed), 2);
        assert_eq!(counters.candidates.load(Ordering::Relaxed), 5);
    }

    #[test]
    fn reporter_start_finish_is_clean_before_the_initial_delay() {
        // Fast path: finish before the first tick prints anything.
        let reporter = ProgressReporter::start();
        reporter.counters().add_project();
        reporter.finish();
    }
}
