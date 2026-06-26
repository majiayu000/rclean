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
        check_any_anchor(
            "homebrew.downloads",
            homebrew_download_anchors(&home),
            "no Homebrew download cache detected",
        ),
        check_anchor(
            "dart.pub_hosted_cache",
            home.join(".pub-cache").join("hosted"),
            "no Dart pub hosted cache detected",
        ),
        check_anchor(
            "dart.pub_git_cache",
            home.join(".pub-cache").join("git"),
            "no Dart pub git cache detected",
        ),
        check_anchor(
            "go.module_download_cache",
            home.join("go").join("pkg").join("mod").join("cache"),
            "no Go module cache detected",
        ),
        check_anchor(
            "go.module_cache",
            home.join("go").join("pkg").join("mod"),
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
            "node.npm_transient",
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
        check_anchor(
            "ruby.bundle_compact_index",
            home.join(".bundle").join("cache").join("compact_index"),
            "no Bundler compact index detected",
        ),
        check_anchor(
            "cloud.kube_cache",
            home.join(".kube").join("cache"),
            "no Kubernetes cache detected",
        ),
        check_anchor(
            "cloud.gcloud_logs",
            home.join(".config").join("gcloud").join("logs"),
            "no gcloud logs detected",
        ),
        check_anchor(
            "editor.vscode_obsolete_extension",
            home.join(".vscode").join("extensions"),
            "no VS Code extensions detected",
        ),
        check_anchor(
            "editor.cursor_obsolete_extension",
            home.join(".cursor").join("extensions"),
            "no Cursor extensions detected",
        ),
        check_anchor(
            "claude.old_version",
            home.join(".local")
                .join("share")
                .join("claude")
                .join("versions"),
            "no Claude Code versions detected",
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
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
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
    #[cfg(target_os = "windows")]
    {
        entries.push(check_anchor(
            "pip.cache",
            home.join(".cache"),
            "no XDG cache directory",
        ));
        entries.push(check_anchor(
            "go.build_cache",
            home.join("AppData").join("Local").join("go-build"),
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
        "ai.vllm_compile_cache",
        home.join(".cache").join("vllm").join("torch_compile_cache"),
        "no vLLM compile cache detected",
    ));
    entries.push(check_anchor(
        "ai.whisper_models",
        home.join(".cache").join("whisper"),
        "no Whisper model cache detected",
    ));
    entries.push(check_any_anchor(
        "ai.llama_cpp_cache",
        simple_cache_anchors(&home, "llama.cpp"),
        "no llama.cpp model cache detected",
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
        simple_cache_anchors(&home, "deno"),
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
        entries.push(check_anchor(
            "app.lark_cache",
            home.join("Library")
                .join("Caches")
                .join("LarkInternational"),
            "no Lark/Feishu cache detected",
        ));
        entries.push(skipped_anchor(
            "macos.chrome_code_sign_clone",
            PathBuf::from("/private/var/folders/*/*/X/com.google.Chrome.code_sign_clone"),
            "exact macOS temp candidate not checked by doctor",
        ));
        entries.push(skipped_anchor(
            "macos.remem_dry_run_tmp",
            PathBuf::from("/private/var/folders/*/*/T/remem-dry-run-*"),
            "exact macOS temp candidate not checked by doctor",
        ));
        entries.push(check_anchor(
            "apple.wallpaper_aerial_videos",
            home.join("Library")
                .join("Application Support")
                .join("com.apple.wallpaper")
                .join("aerials"),
            "no macOS aerial wallpaper cache detected",
        ));
        entries.push(skipped_anchor(
            "apple.idleassetsd",
            PathBuf::from("/Library")
                .join("Application Support")
                .join("com.apple.idleassetsd"),
            "system-scope candidate checked by scan --system",
        ));
        entries.push(check_anchor(
            "chrome.opt_guide_model",
            home.join("Library")
                .join("Application Support")
                .join("Google")
                .join("Chrome"),
            "no Chrome app support detected",
        ));
        entries.push(check_anchor(
            "app.lark_update",
            home.join("Library")
                .join("Application Support")
                .join("LarkInternational"),
            "no Lark/Feishu app support detected",
        ));
        entries.push(check_anchor(
            "macos.geod_map_tiles",
            home.join("Library")
                .join("Containers")
                .join("com.apple.geod")
                .join("Data")
                .join("Library")
                .join("Caches")
                .join("com.apple.geod"),
            "no geod map cache detected",
        ));
        entries.push(check_anchor(
            "macos.mediaanalysisd_cache",
            home.join("Library")
                .join("Containers")
                .join("com.apple.mediaanalysisd")
                .join("Data")
                .join("Library")
                .join("Caches"),
            "no mediaanalysisd cache detected",
        ));
        entries.push(check_anchor(
            "macos.mediaanalysisd_tmp",
            home.join("Library")
                .join("Containers")
                .join("com.apple.mediaanalysisd")
                .join("Data")
                .join("tmp"),
            "no mediaanalysisd tmp cache detected",
        ));
        entries.push(check_anchor(
            "editor.vscode_cache",
            home.join("Library")
                .join("Application Support")
                .join("Code"),
            "no VS Code app support detected",
        ));
        entries.push(check_anchor(
            "editor.cursor_cache",
            home.join("Library")
                .join("Application Support")
                .join("Cursor"),
            "no Cursor app support detected",
        ));
        entries.push(check_any_anchor(
            "app.electron_cache",
            ["Notion", "Slack", "LarkInternational"]
                .into_iter()
                .map(|app| home.join("Library").join("Application Support").join(app))
                .collect(),
            "no known Electron app support detected",
        ));
    }
    #[cfg(not(target_os = "macos"))]
    {
        for rule_id in [
            "app.shipit_caches",
            "chrome.cache",
            "chrome.google_updater",
            "app.lark_cache",
            "macos.chrome_code_sign_clone",
            "macos.remem_dry_run_tmp",
            "apple.wallpaper_aerial_videos",
            "apple.idleassetsd",
            "chrome.opt_guide_model",
            "app.lark_update",
            "macos.geod_map_tiles",
            "macos.mediaanalysisd_cache",
            "macos.mediaanalysisd_tmp",
            "editor.vscode_cache",
            "editor.cursor_cache",
            "app.electron_cache",
        ] {
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

#[cfg(target_os = "macos")]
fn skipped_anchor(rule_id: &'static str, anchor: PathBuf, reason: &'static str) -> DoctorEntry {
    DoctorEntry {
        rule_id,
        anchor,
        status: Status::Skipped { reason },
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

/// Canonical anchors for Homebrew's downloaded bottle/source archive
/// cache. Homebrew on macOS normally uses `~/Library/Caches/Homebrew`,
/// while Linux/XDG-style layouts use `~/.cache/Homebrew`.
fn homebrew_download_anchors(home: &std::path::Path) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        vec![
            home.join("Library")
                .join("Caches")
                .join("Homebrew")
                .join("downloads"),
            home.join(".cache").join("Homebrew").join("downloads"),
        ]
    }
    #[cfg(not(target_os = "macos"))]
    {
        vec![home.join(".cache").join("Homebrew").join("downloads")]
    }
}

/// Canonical anchors for simple user cache roots. macOS native is
/// `~/Library/Caches/<tool>` with XDG fallback, Linux uses
/// `~/.cache/<tool>`, and Windows uses `%LOCALAPPDATA%\<tool>`.
fn simple_cache_anchors(home: &std::path::Path, tool: &str) -> Vec<PathBuf> {
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
        vec![home.join("AppData").join("Local").join(tool)]
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
        // v0.3 Phase 2 + Python + Deno + Puppeteer + AI/ML had 26 entries;
        // #116 conservative user/app cache coverage adds 10 more;
        // #117 macOS whole-machine/app cache coverage adds 7 more anchors
        // plus the Go modcache root cleanup rule. #160/#162 add 6 more
        // exact-anchor global cache rules. #158 adds one system-scope
        // report-only rule.
        assert_eq!(report.total_count(), 53);
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
