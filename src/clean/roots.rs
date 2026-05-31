use std::path::{Path, PathBuf};

use crate::error::CleanError;

pub fn check_broad_roots(roots: &[PathBuf]) -> Result<(), CleanError> {
    for root in roots {
        if let Some(canonical) = root.canonicalize().ok().or_else(|| Some(root.clone()))
            && is_broad_root(&canonical)
        {
            return Err(CleanError::Generic(format!(
                "refusing to clean against broad root {}: pass --allow-broad-root to override",
                canonical.display()
            )));
        }
    }
    Ok(())
}

fn is_broad_root(path: &Path) -> bool {
    if path.has_root()
        && !path
            .components()
            .any(|component| matches!(component, std::path::Component::Normal(_)))
    {
        return true;
    }

    let broad: &[&str] = &[
        "/",
        "/etc",
        "/usr",
        "/var",
        "/opt",
        "/tmp",
        "/System",
        "/Library",
        "/private",
        "/Users",
        "/home",
        "/root",
        // macOS canonical forms (paths under /private)
        "/private/etc",
        "/private/var",
        "/private/tmp",
        "C:\\",
        "C:\\Windows",
        "C:\\Users",
        "C:\\Program Files",
        "C:\\Program Files (x86)",
    ];

    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from)
        && path == home
    {
        return true;
    }
    if let Some(userprofile) = std::env::var_os("USERPROFILE").map(PathBuf::from)
        && path == userprofile
    {
        return true;
    }
    let path_str = path.to_string_lossy();
    broad.iter().any(|b| path_str.as_ref() == *b)
}
