//! `rclean doctor` — diagnostic for which global-cache rules are
//! applicable on this machine.
//!
//! Each Phase 1 global-path rule has a canonical anchor directory
//! (`~/.cargo`, `~/.gradle`, `~/Library/Developer`, ...). Doctor
//! reports per-rule whether that anchor exists, so the user can
//! see at a glance what `rclean scan --home` will actually touch.
//!
//! No filesystem writes, no subprocess spawns. Pure dir-exists
//! checks. Safe to run on any machine, including CI.
//!
//! See `docs/specs/v0.2-developer-mole.md` §4.3.

use std::path::PathBuf;

#[derive(Debug)]
pub struct DoctorReport {
    pub entries: Vec<DoctorEntry>,
}

#[derive(Debug)]
pub struct DoctorEntry {
    pub rule_id: &'static str,
    pub anchor: PathBuf,
    pub status: Status,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Applicable,
    Skipped { reason: &'static str },
}

impl DoctorReport {
    pub fn applicable_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.status, Status::Applicable))
            .count()
    }

    pub fn total_count(&self) -> usize {
        self.entries.len()
    }
}

pub fn diagnose() -> DoctorReport {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return DoctorReport {
            entries: Vec::new(),
        };
    };

    // Cross-platform rules. Each anchor is the directory the rule's
    // classifier checks via parent_ends_with — its existence is a
    // necessary (not sufficient) condition for the rule to fire.
    let mut entries = vec![
        check_anchor(
            "cargo.registry_cache",
            home.join(".cargo").join("registry"),
            "no Cargo install detected",
        ),
        check_anchor(
            "cargo.git_db",
            home.join(".cargo").join("git"),
            "no Cargo git deps detected",
        ),
        check_anchor(
            "go.module_download_cache",
            home.join("go").join("pkg").join("mod").join("cache"),
            "no Go module cache detected",
        ),
        check_anchor(
            "gradle.caches",
            home.join(".gradle"),
            "no Gradle install detected",
        ),
        check_anchor(
            "maven.local_repo",
            home.join(".m2"),
            "no Maven install detected",
        ),
        check_anchor(
            "node.npm_cacache",
            home.join(".npm"),
            "no npm install detected",
        ),
        check_anchor(
            "bun.cache",
            home.join(".bun").join("install"),
            "no bun install cache detected",
        ),
        check_anchor(
            "pre_commit.cache",
            home.join(".cache"),
            "no XDG cache directory",
        ),
    ];

    let mut pnpm_anchors = vec![home.join(".pnpm-store")];
    #[cfg(target_os = "macos")]
    {
        pnpm_anchors.push(home.join("Library").join("pnpm").join("store"));
        pnpm_anchors.push(
            home.join("Library")
                .join("Caches")
                .join("pnpm")
                .join("store"),
        );
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        pnpm_anchors.push(home.join(".local").join("share").join("pnpm").join("store"));
    }
    #[cfg(target_os = "windows")]
    {
        pnpm_anchors.push(
            home.join("AppData")
                .join("Local")
                .join("pnpm")
                .join("store"),
        );
    }
    entries.push(check_any_anchor(
        "node.pnpm_store",
        pnpm_anchors,
        "no pnpm store detected",
    ));

    // pip uses different anchors per platform.
    #[cfg(target_os = "macos")]
    {
        entries.push(check_anchor(
            "pip.cache",
            home.join("Library").join("Caches"),
            "no Library/Caches directory",
        ));
        entries.push(check_anchor(
            "go.build_cache",
            home.join("Library").join("Caches").join("go-build"),
            "no Go build cache detected",
        ));
    }
    #[cfg(not(target_os = "macos"))]
    {
        entries.push(check_anchor(
            "pip.cache",
            home.join(".cache"),
            "no XDG cache directory",
        ));
        entries.push(check_anchor(
            "go.build_cache",
            home.join(".cache").join("go-build"),
            "no Go build cache detected",
        ));
    }

    // AI / ML model caches (#102). All three rules anchor under
    // `~/.cache/...` and `~/.ollama/...` on every platform.
    entries.push(check_anchor(
        "ai.huggingface_hub",
        home.join(".cache").join("huggingface").join("hub"),
        "no HuggingFace cache detected",
    ));
    entries.push(check_anchor(
        "ai.torch_hub",
        home.join(".cache").join("torch").join("hub"),
        "no PyTorch hub cache detected",
    ));
    entries.push(check_anchor(
        "ai.ollama_models",
        home.join(".ollama").join("models"),
        "no Ollama install detected",
    ));

    // Python global tooling caches (#101). uv, Poetry, and pipx each
    // resolve to either the native macOS path or the XDG override —
    // real users hit both, so doctor accepts either anchor.
    entries.push(check_any_anchor(
        "python.uv_cache",
        python_cache_anchors(&home, "uv"),
        "no uv install detected",
    ));
    entries.push(check_any_anchor(
        "python.poetry_cache",
        python_cache_anchors(&home, "pypoetry"),
        "no Poetry install detected",
    ));
    entries.push(check_any_anchor(
        "python.pipx_cache",
        python_cache_anchors(&home, "pipx"),
        "no pipx install detected",
    ));

    // Deno's cache can be native macOS or XDG-style, depending on
    // platform and user environment.
    entries.push(check_any_anchor(
        "js.deno_cache",
        deno_cache_anchors(&home),
        "no Deno install detected",
    ));

    // Puppeteer keeps Chrome for Testing downloads in a global cache.
    entries.push(check_any_anchor(
        "browser.puppeteer",
        browser_cache_anchors(&home, "puppeteer"),
        "no Puppeteer install detected",
    ));

    // macOS-only rules. On non-macOS the anchor never exists, so the
    // entry is reported as Skipped with a platform reason — gives
    // Linux users an accurate "this rule doesn't apply here" instead
    // of hiding it.
    #[cfg(target_os = "macos")]
    {
        entries.push(check_anchor(
            "node.yarn_cache",
            home.join("Library").join("Caches"),
            "no Library/Caches directory",
        ));
        entries.push(check_anchor(
            "xcode.derived_data",
            home.join("Library").join("Developer").join("Xcode"),
            "no Xcode install detected",
        ));
        entries.push(check_anchor(
            "xcode.simulators",
            home.join("Library").join("Developer"),
            "no Xcode install detected",
        ));
    }
    #[cfg(not(target_os = "macos"))]
    {
        entries.push(DoctorEntry {
            rule_id: "node.yarn_cache",
            anchor: PathBuf::from("(macOS only)"),
            status: Status::Skipped {
                reason: "rule only applies on macOS",
            },
        });
        entries.push(DoctorEntry {
            rule_id: "xcode.derived_data",
            anchor: PathBuf::from("(macOS only)"),
            status: Status::Skipped {
                reason: "rule only applies on macOS",
            },
        });
        entries.push(DoctorEntry {
            rule_id: "xcode.simulators",
            anchor: PathBuf::from("(macOS only)"),
            status: Status::Skipped {
                reason: "rule only applies on macOS",
            },
        });
    }

    // Playwright lives in `~/Library/Caches/ms-playwright` on macOS
    // and `~/.cache/ms-playwright` on Linux. On Windows the layout is
    // different and v0.3 doesn't support it — report Skipped.
    #[cfg(target_os = "macos")]
    {
        entries.push(check_anchor(
            "playwright.browsers",
            home.join("Library").join("Caches").join("ms-playwright"),
            "no Playwright browsers detected",
        ));
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        entries.push(check_anchor(
            "playwright.browsers",
            home.join(".cache").join("ms-playwright"),
            "no Playwright browsers detected",
        ));
    }
    #[cfg(target_os = "windows")]
    {
        entries.push(DoctorEntry {
            rule_id: "playwright.browsers",
            anchor: PathBuf::from("(macOS / Linux only)"),
            status: Status::Skipped {
                reason: "rule only applies on macOS and Linux",
            },
        });
    }

    // v0.3 Phase 2: GUI app caches under ~/Library/* (macOS only).
    // Each rule anchors on the candidate's parent directory, so
    // doctor checks whether that parent exists at all.
    #[cfg(target_os = "macos")]
    {
        entries.push(check_anchor(
            "app.shipit_caches",
            home.join("Library").join("Caches"),
            "no Library/Caches directory",
        ));
        entries.push(check_anchor(
            "chrome.cache",
            home.join("Library").join("Caches").join("Google"),
            "no Chrome cache detected",
        ));
        entries.push(check_anchor(
            "chrome.google_updater",
            home.join("Library")
                .join("Application Support")
                .join("Google"),
            "no Google app data detected",
        ));
    }
    #[cfg(not(target_os = "macos"))]
    {
        for rule_id in ["app.shipit_caches", "chrome.cache", "chrome.google_updater"] {
            entries.push(DoctorEntry {
                rule_id,
                anchor: PathBuf::from("(macOS only)"),
                status: Status::Skipped {
                    reason: "rule only applies on macOS",
                },
            });
        }
    }

    DoctorReport { entries }
}

fn check_anchor(
    rule_id: &'static str,
    anchor: PathBuf,
    missing_reason: &'static str,
) -> DoctorEntry {
    let exists = anchor.is_dir();
    let status = if exists {
        Status::Applicable
    } else {
        Status::Skipped {
            reason: missing_reason,
        }
    };
    DoctorEntry {
        rule_id,
        anchor,
        status,
    }
}

/// Canonical anchors for a browser-automation cache directory.
/// macOS defaults to `~/Library/Caches/<tool>` but may also use
/// `~/.cache/<tool>` as an XDG fallback.
fn browser_cache_anchors(home: &std::path::Path, tool: &str) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        vec![
            home.join("Library").join("Caches").join(tool),
            home.join(".cache").join(tool),
        ]
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        vec![home.join(".cache").join(tool)]
    }
    #[cfg(target_os = "windows")]
    {
        vec![home.join("AppData").join("Local").join(tool).join("Cache")]
    }
}

/// Canonical anchors for Deno's remote-dependency cache. macOS native
/// is `~/Library/Caches/deno`; Linux uses `~/.cache/deno`; Windows
/// uses `%LOCALAPPDATA%\deno`.
fn deno_cache_anchors(home: &std::path::Path) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        vec![
            home.join("Library").join("Caches").join("deno"),
            home.join(".cache").join("deno"),
        ]
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        vec![home.join(".cache").join("deno")]
    }
    #[cfg(target_os = "windows")]
    {
        vec![home.join("AppData").join("Local").join("deno")]
    }
}

/// Canonical anchors for a Python toolchain cache directory.
///
/// macOS hosts may resolve to either the native `~/Library/Caches/<tool>`
/// or the XDG override `~/.cache/<tool>` — the empirical dev box behind
/// issue #101 had uv at `~/.cache/uv` while the platformdirs default is
/// `~/Library/Caches/uv`. Linux and Windows have a single canonical path.
fn python_cache_anchors(home: &std::path::Path, tool: &str) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        vec![
            home.join("Library").join("Caches").join(tool),
            home.join(".cache").join(tool),
        ]
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        vec![home.join(".cache").join(tool)]
    }
    #[cfg(target_os = "windows")]
    {
        vec![home.join("AppData").join("Local").join(tool).join("Cache")]
    }
}

fn check_any_anchor(
    rule_id: &'static str,
    anchors: Vec<PathBuf>,
    missing_reason: &'static str,
) -> DoctorEntry {
    if let Some(anchor) = anchors.iter().find(|anchor| anchor.is_dir()) {
        return DoctorEntry {
            rule_id,
            anchor: anchor.clone(),
            status: Status::Applicable,
        };
    }

    DoctorEntry {
        rule_id,
        anchor: anchors
            .into_iter()
            .next()
            .unwrap_or_else(|| PathBuf::from("(unknown)")),
        status: Status::Skipped {
            reason: missing_reason,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::sync::{Mutex, MutexGuard};

    static HOME_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn diagnose_returns_one_entry_per_phase1_global_rule() {
        // Run with a synthetic HOME to make the result deterministic.
        let temp = tempfile::TempDir::new().unwrap();
        // SAFETY: tests in a single binary share process env. The
        // assertion only relies on the total entry count, which is
        // the same regardless of which anchors exist.
        let _restore = with_home(temp.path());

        let report = diagnose();
        // v0.3 Phase 2 + Python + Deno + Puppeteer has 23 entries;
        // AI/ML adds HuggingFace, PyTorch, and Ollama for 26 total.
        assert_eq!(report.total_count(), 26);
    }

    #[test]
    fn diagnose_marks_existing_anchor_applicable() {
        let temp = tempfile::TempDir::new().unwrap();
        // Synthesize ~/.cargo/registry so cargo.registry_cache becomes
        // applicable.
        fs::create_dir_all(temp.path().join(".cargo").join("registry")).unwrap();
        let _restore = with_home(temp.path());

        let report = diagnose();
        let cargo_reg = report
            .entries
            .iter()
            .find(|e| e.rule_id == "cargo.registry_cache")
            .expect("cargo.registry_cache entry should exist");
        assert_eq!(cargo_reg.status, Status::Applicable);
    }

    #[test]
    fn diagnose_marks_missing_anchor_skipped() {
        let temp = tempfile::TempDir::new().unwrap();
        let _restore = with_home(temp.path());

        let report = diagnose();
        let cargo_reg = report
            .entries
            .iter()
            .find(|e| e.rule_id == "cargo.registry_cache")
            .expect("cargo.registry_cache entry should exist");
        assert!(matches!(cargo_reg.status, Status::Skipped { .. }));
    }

    /// Test helper that swaps HOME for the duration of the test and
    /// restores it on drop. Avoids leaking the override into other
    /// tests in the same binary.
    fn with_home(path: &Path) -> HomeGuard {
        let lock = HOME_LOCK.lock().expect("HOME test mutex poisoned");
        let previous = std::env::var_os("HOME");
        // SAFETY: HOME_LOCK serializes every test in this module that
        // mutates the process environment, and Drop restores HOME.
        unsafe {
            std::env::set_var("HOME", path);
        }
        HomeGuard {
            previous,
            _lock: lock,
        }
    }

    struct HomeGuard {
        previous: Option<std::ffi::OsString>,
        _lock: MutexGuard<'static, ()>,
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            // SAFETY: see with_home above.
            unsafe {
                match &self.previous {
                    Some(v) => std::env::set_var("HOME", v),
                    None => std::env::remove_var("HOME"),
                }
            }
        }
    }
}
