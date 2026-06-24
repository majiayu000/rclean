//! Exact-anchor IDE cache/log rules.
//!
//! These rules only target JetBrains IDE and Android Studio rebuildable
//! `caches`/`log` directories under their documented system cache roots,
//! plus macOS `~/Library/Logs/<vendor>/<product-version>` log directories.
//! They deliberately avoid config, plugins, LocalHistory, projects, SDKs,
//! AVDs, and broad Application Support state.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::path_util::path_file_name;
use crate::rules::markers::parent_ends_with;

const JETBRAINS_PRODUCTS: &[&str] = &[
    "IntelliJIdea",
    "PyCharmCE",
    "PyCharm",
    "WebStorm",
    "PhpStorm",
    "RubyMine",
    "CLion",
    "GoLand",
    "DataGrip",
    "DataSpell",
    "Rider",
    "RustRover",
    "Aqua",
    "Writerside",
    "AppCode",
    "MPS",
];

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    if name == "caches" {
        if product_parent_matches(project_dir, Vendor::JetBrains, AnchorKind::System) {
            return Some(ide_candidate(
                path,
                name,
                "jetbrains.system_caches",
                "JetBrains IDE rebuildable system cache directory",
            ));
        }

        if product_parent_matches(project_dir, Vendor::AndroidStudio, AnchorKind::System) {
            return Some(ide_candidate(
                path,
                name,
                "android_studio.system_caches",
                "Android Studio rebuildable system cache directory",
            ));
        }
    }

    if name == "log" {
        if product_parent_matches(project_dir, Vendor::JetBrains, AnchorKind::SystemLog) {
            return Some(ide_candidate(
                path,
                name,
                "jetbrains.logs",
                "JetBrains IDE rebuildable log directory",
            ));
        }

        if product_parent_matches(project_dir, Vendor::AndroidStudio, AnchorKind::SystemLog) {
            return Some(ide_candidate(
                path,
                name,
                "android_studio.logs",
                "Android Studio rebuildable log directory",
            ));
        }
    }

    if product_name_matches(name, Vendor::JetBrains)
        && parent_ends_with(project_dir, &["Library", "Logs", "JetBrains"])
    {
        return Some(ide_candidate(
            path,
            name,
            "jetbrains.logs",
            "JetBrains IDE rebuildable log directory",
        ));
    }

    if product_name_matches(name, Vendor::AndroidStudio)
        && parent_ends_with(project_dir, &["Library", "Logs", "Google"])
    {
        return Some(ide_candidate(
            path,
            name,
            "android_studio.logs",
            "Android Studio rebuildable log directory",
        ));
    }

    None
}

pub(crate) fn is_dynamic_candidate_name(name: &str) -> bool {
    name == "log"
        || product_name_matches(name, Vendor::JetBrains)
        || product_name_matches(name, Vendor::AndroidStudio)
}

#[derive(Debug, Clone, Copy)]
enum Vendor {
    JetBrains,
    AndroidStudio,
}

#[derive(Debug, Clone, Copy)]
enum AnchorKind {
    System,
    SystemLog,
}

fn product_parent_matches(product_dir: &Path, vendor: Vendor, anchor_kind: AnchorKind) -> bool {
    let Some(name) = path_file_name(product_dir) else {
        return false;
    };
    if !product_name_matches(name, vendor) {
        return false;
    }

    let Some(anchor) = product_dir.parent() else {
        return false;
    };
    match (vendor, anchor_kind) {
        (Vendor::JetBrains, AnchorKind::System) => is_jetbrains_system_anchor(anchor),
        (Vendor::JetBrains, AnchorKind::SystemLog) => is_jetbrains_system_log_anchor(anchor),
        (Vendor::AndroidStudio, AnchorKind::System) => is_android_studio_system_anchor(anchor),
        (Vendor::AndroidStudio, AnchorKind::SystemLog) => {
            is_android_studio_system_log_anchor(anchor)
        }
    }
}

fn product_name_matches(name: &str, vendor: Vendor) -> bool {
    match vendor {
        Vendor::JetBrains => JETBRAINS_PRODUCTS.iter().any(|product| {
            name.strip_prefix(product)
                .is_some_and(is_version_like_suffix)
        }),
        Vendor::AndroidStudio => name
            .strip_prefix("AndroidStudio")
            .is_some_and(is_android_studio_suffix),
    }
}

fn is_version_like_suffix(suffix: &str) -> bool {
    let mut chars = suffix.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_digit()
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
}

fn is_android_studio_suffix(suffix: &str) -> bool {
    !suffix.is_empty()
        && suffix.chars().any(|ch| ch.is_ascii_digit())
        && suffix
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
}

fn is_jetbrains_system_anchor(anchor: &Path) -> bool {
    parent_ends_with(anchor, &["Library", "Caches", "JetBrains"])
        || parent_ends_with(anchor, &[".cache", "JetBrains"])
        || parent_ends_with(anchor, &["AppData", "Local", "JetBrains"])
}

fn is_jetbrains_system_log_anchor(anchor: &Path) -> bool {
    parent_ends_with(anchor, &[".cache", "JetBrains"])
        || parent_ends_with(anchor, &["AppData", "Local", "JetBrains"])
}

fn is_android_studio_system_anchor(anchor: &Path) -> bool {
    parent_ends_with(anchor, &["Library", "Caches", "Google"])
        || parent_ends_with(anchor, &[".cache", "Google"])
        || parent_ends_with(anchor, &["AppData", "Local", "Google"])
}

fn is_android_studio_system_log_anchor(anchor: &Path) -> bool {
    parent_ends_with(anchor, &[".cache", "Google"])
        || parent_ends_with(anchor, &["AppData", "Local", "Google"])
}

fn ide_candidate(path: &Path, name: &str, rule_id: &str, reason: &str) -> CandidateDraft {
    CandidateDraft {
        path: path.to_path_buf(),
        name: name.to_string(),
        rule_id: rule_id.to_string(),
        category: Category::Cache,
        safety: Safety::Caution,
        reasons: vec![reason.to_string()],
        warnings: vec!["Close the IDE before removing its caches or logs".to_string()],
        restore_hint: "The IDE will recreate logs and caches on the next launch".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_jetbrains_caches_under_exact_system_anchors() {
        for product in [
            "/Users/me/Library/Caches/JetBrains/IntelliJIdea2024.3",
            "/home/me/.cache/JetBrains/PyCharmCE2025.1",
            "C:/Users/me/AppData/Local/JetBrains/WebStorm2024.2",
        ] {
            let parent = PathBuf::from(product);
            let draft = classify(&parent, "caches", &parent.join("caches"))
                .expect("should classify JetBrains caches");
            assert_eq!(draft.rule_id, "jetbrains.system_caches");
            assert_eq!(draft.safety, Safety::Caution);
        }
    }

    #[test]
    fn classifies_jetbrains_logs_under_exact_log_anchors() {
        let mac_parent = PathBuf::from("/Users/me/Library/Logs/JetBrains");
        let mac_path = mac_parent.join("GoLand2024.3");
        let draft = classify(&mac_parent, "GoLand2024.3", &mac_path)
            .expect("should classify macOS JetBrains logs");
        assert_eq!(draft.rule_id, "jetbrains.logs");

        for product in [
            "/home/me/.cache/JetBrains/CLion2025.1",
            "C:/Users/me/AppData/Local/JetBrains/Rider2024.3",
        ] {
            let parent = PathBuf::from(product);
            let draft = classify(&parent, "log", &parent.join("log"))
                .expect("should classify JetBrains log");
            assert_eq!(draft.rule_id, "jetbrains.logs");
            assert_eq!(draft.safety, Safety::Caution);
        }
    }

    #[test]
    fn classifies_android_studio_caches_and_logs_under_exact_anchors() {
        for product in [
            "/Users/me/Library/Caches/Google/AndroidStudio2024.3",
            "/home/me/.cache/Google/AndroidStudioPreview2025.1",
            "C:/Users/me/AppData/Local/Google/AndroidStudio2024.2",
        ] {
            let parent = PathBuf::from(product);
            let draft = classify(&parent, "caches", &parent.join("caches"))
                .expect("should classify Android Studio caches");
            assert_eq!(draft.rule_id, "android_studio.system_caches");
        }

        let mac_parent = PathBuf::from("/Users/me/Library/Logs/Google");
        let mac_path = mac_parent.join("AndroidStudio2024.3");
        let draft = classify(&mac_parent, "AndroidStudio2024.3", &mac_path)
            .expect("should classify Android Studio macOS logs");
        assert_eq!(draft.rule_id, "android_studio.logs");

        let linux_parent = PathBuf::from("/home/me/.cache/Google/AndroidStudio2024.3");
        let draft = classify(&linux_parent, "log", &linux_parent.join("log"))
            .expect("should classify Android Studio log");
        assert_eq!(draft.rule_id, "android_studio.logs");
    }

    #[test]
    fn rejects_ide_state_outside_exact_cache_and_log_anchors() {
        for (parent, name) in [
            (
                "/Users/me/Library/Application Support/JetBrains/IntelliJIdea2024.3",
                "caches",
            ),
            ("/home/me/.config/JetBrains/PyCharm2024.3", "caches"),
            ("/home/me/.local/share/JetBrains/IntelliJIdea2024.3", "log"),
            (
                "/Users/me/Library/Caches/JetBrains/IntelliJIdea2024.3",
                "plugins",
            ),
            (
                "/Users/me/Library/Caches/JetBrains/IntelliJIdea2024.3",
                "LocalHistory",
            ),
            (
                "/Users/me/Library/Application Support/Google/AndroidStudio2024.3",
                "caches",
            ),
            (
                "/Users/me/Library/Caches/Google/AndroidStudio2024.3",
                "plugins",
            ),
        ] {
            let parent = PathBuf::from(parent);
            assert!(
                classify(&parent, name, &parent.join(name)).is_none(),
                "unexpectedly classified {}/{}",
                parent.display(),
                name
            );
        }
    }
}
