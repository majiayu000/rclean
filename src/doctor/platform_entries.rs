use std::path::Path;
use std::path::PathBuf;

use super::DoctorEntry;
#[cfg(not(target_os = "macos"))]
use super::Status;
#[cfg(not(target_os = "windows"))]
use super::check_anchor;
#[cfg(target_os = "macos")]
use super::{check_any_anchor, skipped_anchor};

pub(super) fn extend(entries: &mut Vec<DoctorEntry>, home: &Path) {
    #[cfg(target_os = "windows")]
    let _ = home;

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
                reason: "rule only applies on macOS".to_string(),
            },
        });
        entries.push(DoctorEntry {
            rule_id: "xcode.derived_data",
            anchor: PathBuf::from("(macOS only)"),
            status: Status::Skipped {
                reason: "rule only applies on macOS".to_string(),
            },
        });
        entries.push(DoctorEntry {
            rule_id: "xcode.simulators",
            anchor: PathBuf::from("(macOS only)"),
            status: Status::Skipped {
                reason: "rule only applies on macOS".to_string(),
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
                reason: "rule only applies on macOS and Linux".to_string(),
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
                    reason: "rule only applies on macOS".to_string(),
                },
            });
        }
    }
}
