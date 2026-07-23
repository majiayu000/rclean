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
/// Last-resort fallback to a namespaced `./.rclean-plans/` directory
/// only when none of the home environment variables are set — mainly
/// for CI sandboxes with stripped environments. It mirrors the
/// graveyard's `./.rclean-graveyard` fallback: still relative, but a
/// single ignorable directory rather than timestamped files dropped
/// loose into the working directory, which is the litter #349 is
/// about.
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
    PathBuf::from(".rclean-plans")
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::with_env_vars;

    const ENV_KEYS: [&str; 4] = ["XDG_STATE_HOME", "LOCALAPPDATA", "HOME", "USERPROFILE"];

    fn cleared() -> Vec<(&'static str, Option<&'static str>)> {
        ENV_KEYS.iter().map(|key| (*key, None)).collect()
    }

    /// Environment mutation is process-global and `cargo test --lib`
    /// runs tests in parallel, so every case runs inside one test that
    /// holds the crate-wide env lock for its whole duration.
    #[test]
    fn resolves_plans_dir_by_environment_precedence() {
        let guard = with_env_vars(&cleared());

        // XDG_STATE_HOME wins when set.
        guard.set(&[
            ("XDG_STATE_HOME", Some("/tmp/state")),
            ("HOME", Some("/home/user")),
        ]);
        assert_eq!(
            default_plans_dir(),
            PathBuf::from("/tmp/state").join("rclean").join("plans")
        );

        // An empty value is ignored, not treated as a valid root.
        guard.set(&cleared());
        guard.set(&[("XDG_STATE_HOME", Some("")), ("HOME", Some("/home/user"))]);
        assert_eq!(
            default_plans_dir(),
            PathBuf::from("/home/user")
                .join(".local")
                .join("state")
                .join("rclean")
                .join("plans")
        );

        // LOCALAPPDATA covers Windows.
        guard.set(&cleared());
        guard.set(&[("LOCALAPPDATA", Some("C:\\Users\\x\\AppData\\Local"))]);
        assert_eq!(
            default_plans_dir(),
            PathBuf::from("C:\\Users\\x\\AppData\\Local")
                .join("rclean")
                .join("plans")
        );

        // USERPROFILE is the last named fallback.
        guard.set(&cleared());
        guard.set(&[("USERPROFILE", Some("C:\\Users\\x"))]);
        assert_eq!(
            default_plans_dir(),
            PathBuf::from("C:\\Users\\x")
                .join("AppData")
                .join("Local")
                .join("rclean")
                .join("plans")
        );

        // A stripped environment still resolves, and to a namespaced
        // directory rather than a loose file in the working directory.
        guard.set(&cleared());
        assert_eq!(default_plans_dir(), PathBuf::from(".rclean-plans"));
    }
}
