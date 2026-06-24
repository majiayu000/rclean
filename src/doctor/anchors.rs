use std::path::{Path, PathBuf};

/// Canonical anchors for a browser-automation cache directory.
/// macOS defaults to `~/Library/Caches/<tool>` but may also use
/// `~/.cache/<tool>` as an XDG fallback.
pub(super) fn browser_cache_anchors(home: &Path, tool: &str) -> Vec<PathBuf> {
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
pub(super) fn homebrew_download_anchors(home: &Path) -> Vec<PathBuf> {
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

/// Canonical anchors for Deno's remote-dependency cache. macOS native
/// is `~/Library/Caches/deno`; Linux uses `~/.cache/deno`; Windows
/// uses `%LOCALAPPDATA%\deno`.
pub(super) fn deno_cache_anchors(home: &Path) -> Vec<PathBuf> {
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

/// Canonical anchors for exact IDE system cache roots.
pub(super) fn ide_system_anchors(home: &Path, vendor: &str) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        vec![home.join("Library").join("Caches").join(vendor)]
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        vec![home.join(".cache").join(vendor)]
    }
    #[cfg(target_os = "windows")]
    {
        vec![home.join("AppData").join("Local").join(vendor)]
    }
}

/// Canonical anchors for exact IDE log roots.
pub(super) fn ide_log_anchors(home: &Path, vendor: &str) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        vec![home.join("Library").join("Logs").join(vendor)]
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        vec![home.join(".cache").join(vendor)]
    }
    #[cfg(target_os = "windows")]
    {
        vec![home.join("AppData").join("Local").join(vendor)]
    }
}

/// Canonical anchors for a Python toolchain cache directory.
///
/// macOS hosts may resolve to either the native `~/Library/Caches/<tool>`
/// or the XDG override `~/.cache/<tool>`; Linux and Windows have a single
/// canonical path.
pub(super) fn python_cache_anchors(home: &Path, tool: &str) -> Vec<PathBuf> {
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
