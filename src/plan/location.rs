//! Default on-disk location for generated ActionPlans (spec:
//! `specs/GH349/tech.md`).
//!
//! Plans are proposals a user reviews and then replays, so they belong
//! in the user's *state* directory rather than in whatever working
//! directory the command happened to run from. The environment
//! precedence deliberately mirrors [`crate::graveyard::default_root`]
//! so both agree on which variable wins; the directory differs because
//! a plan is regenerable while a grave holds the only copy of deleted
//! bytes.

use std::path::PathBuf;

/// `<state>/rclean/plans/`, resolved per platform:
///
///   Linux/macOS: `$XDG_STATE_HOME/rclean/plans/` or
///                `$HOME/.local/state/rclean/plans/`
///   Windows:     `%LOCALAPPDATA%\rclean\plans\` or
///                `%USERPROFILE%\AppData\Local\rclean\plans\`
///
/// Last-resort fallback to the current directory only when none of the
/// home environment variables are set — mainly for CI sandboxes with
/// stripped environments, matching the graveyard's fallback posture.
pub fn default_plans_dir() -> PathBuf {
    if let Some(dir) = non_empty_env("XDG_STATE_HOME") {
        return PathBuf::from(dir).join("rclean").join("plans");
    }
    if let Some(dir) = non_empty_env("LOCALAPPDATA") {
        return PathBuf::from(dir).join("rclean").join("plans");
    }
    if let Some(home) = non_empty_env("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("rclean")
            .join("plans");
    }
    if let Some(profile) = non_empty_env("USERPROFILE") {
        return PathBuf::from(profile)
            .join("AppData")
            .join("Local")
            .join("rclean")
            .join("plans");
    }
    PathBuf::new()
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Environment mutation is process-global, so every case runs
    /// serially inside one test rather than as parallel tests.
    #[test]
    fn resolves_plans_dir_by_environment_precedence() {
        let saved: Vec<(&str, Option<String>)> =
            ["XDG_STATE_HOME", "LOCALAPPDATA", "HOME", "USERPROFILE"]
                .into_iter()
                .map(|key| (key, std::env::var(key).ok()))
                .collect();

        let clear_all = || {
            for key in ["XDG_STATE_HOME", "LOCALAPPDATA", "HOME", "USERPROFILE"] {
                unsafe { std::env::remove_var(key) };
            }
        };

        // XDG_STATE_HOME wins when set.
        clear_all();
        unsafe { std::env::set_var("XDG_STATE_HOME", "/tmp/state") };
        unsafe { std::env::set_var("HOME", "/home/user") };
        assert_eq!(
            default_plans_dir(),
            PathBuf::from("/tmp/state").join("rclean").join("plans")
        );

        // An empty value is ignored, not treated as a valid root.
        clear_all();
        unsafe { std::env::set_var("XDG_STATE_HOME", "") };
        unsafe { std::env::set_var("HOME", "/home/user") };
        assert_eq!(
            default_plans_dir(),
            PathBuf::from("/home/user")
                .join(".local")
                .join("state")
                .join("rclean")
                .join("plans")
        );

        // LOCALAPPDATA covers Windows.
        clear_all();
        unsafe { std::env::set_var("LOCALAPPDATA", "C:\\Users\\x\\AppData\\Local") };
        assert_eq!(
            default_plans_dir(),
            PathBuf::from("C:\\Users\\x\\AppData\\Local")
                .join("rclean")
                .join("plans")
        );

        // USERPROFILE is the last named fallback.
        clear_all();
        unsafe { std::env::set_var("USERPROFILE", "C:\\Users\\x") };
        assert_eq!(
            default_plans_dir(),
            PathBuf::from("C:\\Users\\x")
                .join("AppData")
                .join("Local")
                .join("rclean")
                .join("plans")
        );

        // No home environment at all still yields a usable relative
        // path instead of failing to resolve.
        clear_all();
        assert_eq!(default_plans_dir(), PathBuf::new());

        for (key, value) in saved {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}
