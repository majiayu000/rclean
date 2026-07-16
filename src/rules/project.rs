use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::Path;

use super::ide_caches::is_dynamic_candidate_name as is_ide_dynamic_candidate_name;
use super::macos_system::is_dynamic_candidate_name as is_macos_system_dynamic_candidate_name;
use super::markers::{
    has_marker, has_prefixed_marker, is_dotnet_project, is_python_project, is_ruby_project,
    package_mentions,
};
use super::node_global::is_pnpm_store_version_name;
use super::user_tool_caches::is_dynamic_candidate_name as is_user_tool_dynamic_candidate_name;

const PROJECT_ROOT_SNAPSHOT_LIMIT: usize = 64;
const PROJECT_MARKERS: [&str; 14] = [
    "package.json",
    "Cargo.toml",
    "go.mod",
    "Podfile",
    "pyproject.toml",
    "requirements.txt",
    "setup.py",
    "Pipfile",
    "Gemfile",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "pubspec.yaml",
    "composer.json",
];

struct ProjectRootSnapshot {
    entry_names: Vec<OsString>,
    file_names: HashSet<OsString>,
}

impl ProjectRootSnapshot {
    fn read(dir: &Path) -> Option<Self> {
        let entries = fs::read_dir(dir).ok()?;
        let mut entry_names = Vec::new();
        let mut file_names = HashSet::new();

        for entry in entries {
            let entry = entry.ok()?;
            if entry_names.len() == PROJECT_ROOT_SNAPSHOT_LIMIT {
                return None;
            }

            let file_type = entry.file_type().ok()?;
            let name = entry.file_name();
            if file_type.is_file() || (file_type.is_symlink() && entry.path().is_file()) {
                file_names.insert(name.clone());
            }
            entry_names.push(name);
        }

        Some(Self {
            entry_names,
            file_names,
        })
    }

    fn has_file(&self, marker: &str) -> bool {
        self.file_names.contains(OsStr::new(marker))
    }

    fn has_prefix(&self, prefix: &str) -> bool {
        self.entry_names
            .iter()
            .any(|name| name.to_str().is_some_and(|name| name.starts_with(prefix)))
    }

    fn has_extension(&self, extension: &str) -> bool {
        self.entry_names.iter().any(|name| {
            Path::new(name)
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        })
    }
}

pub fn is_candidate_name(name: &str) -> bool {
    matches!(
        name,
        "node_modules"
            | ".next"
            | ".turbo"
            | ".vite"
            | ".parcel-cache"
            | ".venv"
            | "venv"
            | "__pycache__"
            | ".pytest_cache"
            | ".mypy_cache"
            | ".ruff_cache"
            | ".tox"
            | "target"
            | "mod"
            | "vendor"
            | "Pods"
            | "coverage"
            | ".gradle"
            | ".dart_tool"
            | "bin"
            | "obj"
            | ".bundle"
            | "build"
            | "dist"
            | "out"
            | "DerivedData"
            | "cache"
            | "download"
            | "db"
            | "downloads"
            | ".downloadIntermediates"
            | "build-cache"
            | "hosted"
            | "git"
            | "go-build"
            | "store"
            | "_cacache"
            | "_npx"
            | "_logs"
            | "_prebuilds"
            | "Yarn"
            | "pip"
            | "hub"
            | "torch_compile_cache"
            | "whisper"
            | "llama.cpp"
            | "models"
            | "puppeteer"
            | "pre-commit"
            | "deno"
            | "uv"
            | "pypoetry"
            | "pipx"
            | "caches"
            | "repository"
            | "CoreSimulator"
            | "ms-playwright"
            | "Chrome"
            | "GoogleUpdater"
            | "LarkInternational"
            | "com.google.Chrome.code_sign_clone"
            | "videos"
            | "OptGuideOnDeviceModel"
            | "update"
            | "MapTiles"
            | "MediaCache"
            | "com.apple.mediaanalysisd"
            | "com.apple.idleassetsd"
            | "compact_index"
            | "logs"
            | "log"
            | "Cache"
            | "CachedData"
            | "Code Cache"
            | "GPUCache"
    ) || is_pnpm_store_version_name(name)
        || is_shipit_candidate_name(name)
        || is_ide_dynamic_candidate_name(name)
        || is_macos_system_dynamic_candidate_name(name)
        || is_user_tool_dynamic_candidate_name(name)
}

fn is_shipit_candidate_name(name: &str) -> bool {
    name.ends_with(".ShipIt") && name.len() > ".ShipIt".len()
}

/// Returns true for rule ids whose classifier intentionally targets
/// paths inside the user runtime/system tree (e.g. `~/Library/...`).
/// `apply_path_safety` skips the generic runtime-path block for these.
pub fn is_global_rule(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "xcode.derived_data"
            | "cargo.registry_cache"
            | "cargo.git_db"
            | "homebrew.downloads"
            | "android_sdk.download_intermediates"
            | "android_sdk.legacy_build_cache"
            | "jetbrains.system_caches"
            | "jetbrains.logs"
            | "android_studio.system_caches"
            | "android_studio.logs"
            | "dart.pub_hosted_cache"
            | "dart.pub_git_cache"
            | "node.npm_cacache"
            | "node.yarn_cache"
            | "node.pnpm_store"
            | "ai.huggingface_hub"
            | "ai.torch_hub"
            | "ai.vllm_compile_cache"
            | "ai.whisper_models"
            | "ai.llama_cpp_cache"
            | "ai.ollama_models"
            | "browser.puppeteer"
            | "js.deno_cache"
            | "pip.cache"
            | "python.uv_cache"
            | "python.poetry_cache"
            | "python.pipx_cache"
            | "gradle.caches"
            | "maven.local_repo"
            | "xcode.simulators"
            | "go.module_download_cache"
            | "go.module_cache"
            | "go.build_cache"
            | "bun.cache"
            | "pre_commit.cache"
            | "playwright.browsers"
            | "app.shipit_caches"
            | "chrome.cache"
            | "chrome.google_updater"
            | "app.lark_cache"
            | "macos.chrome_code_sign_clone"
            | "macos.remem_dry_run_tmp"
            | "apple.wallpaper_aerial_videos"
            | "chrome.opt_guide_model"
            | "app.lark_update"
            | "macos.geod_map_tiles"
            | "macos.mediaanalysisd_cache"
            | "macos.mediaanalysisd_tmp"
            | "apple.idleassetsd"
            | "node.npm_transient"
            | "ruby.bundle_compact_index"
            | "cloud.kube_cache"
            | "cloud.gcloud_logs"
            | "editor.vscode_cache"
            | "editor.cursor_cache"
            | "editor.vscode_obsolete_extension"
            | "editor.cursor_obsolete_extension"
            | "claude.old_version"
            | "app.electron_cache"
    )
}

pub fn is_project_marker_name(name: &str) -> bool {
    matches!(
        name,
        "package.json"
            | "Cargo.toml"
            | "go.mod"
            | "Podfile"
            | "pyproject.toml"
            | "requirements.txt"
            | "setup.py"
            | "Pipfile"
            | "Gemfile"
            | "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "pubspec.yaml"
            | "composer.json"
    )
}

pub fn detect_project_kind(dir: &Path) -> (String, Vec<String>) {
    match ProjectRootSnapshot::read(dir) {
        Some(snapshot) => detect_project_kind_from_snapshot(dir, &snapshot),
        None => detect_project_kind_targeted(dir),
    }
}

fn detect_project_kind_from_snapshot(
    dir: &Path,
    snapshot: &ProjectRootSnapshot,
) -> (String, Vec<String>) {
    let markers = PROJECT_MARKERS
        .iter()
        .filter(|marker| snapshot.has_file(marker))
        .map(|marker| (*marker).to_string())
        .collect();

    if snapshot.has_file("package.json") {
        let package_json = fs::read_to_string(dir.join("package.json")).ok();
        if snapshot.has_prefix("next.config.")
            || package_content_mentions(package_json.as_deref(), "next")
        {
            return ("Next.js".to_string(), markers);
        }
        if snapshot.has_prefix("vite.config.")
            || package_content_mentions(package_json.as_deref(), "vite")
        {
            return ("Vite".to_string(), markers);
        }
        return ("Node.js".to_string(), markers);
    }
    if snapshot.has_file("Cargo.toml") {
        return ("Rust".to_string(), markers);
    }
    if ["pyproject.toml", "requirements.txt", "setup.py", "Pipfile"]
        .iter()
        .any(|marker| snapshot.has_file(marker))
    {
        return ("Python".to_string(), markers);
    }
    if snapshot.has_file("go.mod") {
        return ("Go".to_string(), markers);
    }
    if snapshot.has_file("Podfile") {
        return ("iOS".to_string(), markers);
    }
    if ["sln", "csproj", "fsproj"]
        .iter()
        .any(|extension| snapshot.has_extension(extension))
    {
        return (".NET".to_string(), markers);
    }
    if snapshot.has_file("Gemfile") {
        return ("Ruby".to_string(), markers);
    }
    if snapshot.has_file("pom.xml") {
        return ("Java (Maven)".to_string(), markers);
    }
    if snapshot.has_file("build.gradle") || snapshot.has_file("build.gradle.kts") {
        return ("Java (Gradle)".to_string(), markers);
    }
    if snapshot.has_file("pubspec.yaml") {
        return ("Flutter/Dart".to_string(), markers);
    }

    ("Unknown".to_string(), markers)
}

fn package_content_mentions(raw: Option<&str>, dependency: &str) -> bool {
    let needle = format!("\"{dependency}\"");
    raw.is_some_and(|raw| raw.contains(&needle))
}

fn detect_project_kind_targeted(dir: &Path) -> (String, Vec<String>) {
    let mut markers = Vec::new();

    for marker in PROJECT_MARKERS {
        if has_marker(dir, marker) {
            markers.push(marker.to_string());
        }
    }

    if has_marker(dir, "package.json") {
        if has_prefixed_marker(dir, "next.config.") || package_mentions(dir, "next") {
            return ("Next.js".to_string(), markers);
        }
        if has_prefixed_marker(dir, "vite.config.") || package_mentions(dir, "vite") {
            return ("Vite".to_string(), markers);
        }
        return ("Node.js".to_string(), markers);
    }
    if has_marker(dir, "Cargo.toml") {
        return ("Rust".to_string(), markers);
    }
    if is_python_project(dir) {
        return ("Python".to_string(), markers);
    }
    if has_marker(dir, "go.mod") {
        return ("Go".to_string(), markers);
    }
    if has_marker(dir, "Podfile") {
        return ("iOS".to_string(), markers);
    }
    if is_dotnet_project(dir) {
        return (".NET".to_string(), markers);
    }
    if is_ruby_project(dir) {
        return ("Ruby".to_string(), markers);
    }
    if has_marker(dir, "pom.xml") {
        return ("Java (Maven)".to_string(), markers);
    }
    if has_marker(dir, "build.gradle") || has_marker(dir, "build.gradle.kts") {
        return ("Java (Gradle)".to_string(), markers);
    }
    if has_marker(dir, "pubspec.yaml") {
        return ("Flutter/Dart".to_string(), markers);
    }

    ("Unknown".to_string(), markers)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    use super::{
        PROJECT_ROOT_SNAPSHOT_LIMIT, ProjectRootSnapshot, detect_project_kind,
        detect_project_kind_targeted, is_candidate_name,
    };

    type ProjectKindCase<'a> = (&'a [(&'a str, &'a [u8])], &'a str, &'a [&'a str]);

    fn write_marker(root: &Path, marker: &str) {
        fs::write(root.join(marker), b"marker").unwrap();
    }

    fn assert_matches_targeted(root: &Path, expected_kind: &str, expected_markers: &[&str]) {
        let expected = (
            expected_kind.to_string(),
            expected_markers
                .iter()
                .map(|marker| (*marker).to_string())
                .collect(),
        );
        assert_eq!(detect_project_kind_targeted(root), expected);
        assert_eq!(detect_project_kind(root), expected);
    }

    #[test]
    fn targeted_detector_classifies_every_project_kind() {
        let cases: &[ProjectKindCase<'_>] = &[
            (
                &[("package.json", br#"{"next":"latest"}"#)],
                "Next.js",
                &["package.json"],
            ),
            (
                &[("package.json", br#"{"vite":"latest"}"#)],
                "Vite",
                &["package.json"],
            ),
            (&[("package.json", b"{}")], "Node.js", &["package.json"]),
            (&[("Cargo.toml", b"[package]")], "Rust", &["Cargo.toml"]),
            (
                &[("pyproject.toml", b"[project]")],
                "Python",
                &["pyproject.toml"],
            ),
            (&[("go.mod", b"module example")], "Go", &["go.mod"]),
            (&[("Podfile", b"platform :ios")], "iOS", &["Podfile"]),
            (
                &[("Gemfile", b"source 'https://example.test'")],
                "Ruby",
                &["Gemfile"],
            ),
            (&[("pom.xml", b"<project />")], "Java (Maven)", &["pom.xml"]),
            (
                &[("build.gradle.kts", b"plugins {}")],
                "Java (Gradle)",
                &["build.gradle.kts"],
            ),
            (
                &[("pubspec.yaml", b"name: example")],
                "Flutter/Dart",
                &["pubspec.yaml"],
            ),
        ];

        for (files, expected_kind, expected_markers) in cases {
            let temp = TempDir::new().unwrap();
            for (name, contents) in *files {
                fs::write(temp.path().join(name), contents).unwrap();
            }
            assert_matches_targeted(temp.path(), expected_kind, expected_markers);
        }

        let dotnet = TempDir::new().unwrap();
        fs::create_dir(dotnet.path().join("example.csproj")).unwrap();
        assert_matches_targeted(dotnet.path(), ".NET", &[]);

        let unknown = TempDir::new().unwrap();
        assert_matches_targeted(unknown.path(), "Unknown", &[]);
    }

    #[test]
    fn targeted_detector_preserves_marker_order_and_kind_priority() {
        let temp = TempDir::new().unwrap();
        let markers = [
            "package.json",
            "Cargo.toml",
            "go.mod",
            "Podfile",
            "pyproject.toml",
            "requirements.txt",
            "setup.py",
            "Pipfile",
            "Gemfile",
            "pom.xml",
            "build.gradle",
            "build.gradle.kts",
            "pubspec.yaml",
            "composer.json",
        ];
        for marker in markers {
            write_marker(temp.path(), marker);
        }

        assert_matches_targeted(temp.path(), "Node.js", &markers);
    }

    #[test]
    fn targeted_detector_preserves_entry_name_and_exact_file_semantics() {
        let next = TempDir::new().unwrap();
        write_marker(next.path(), "package.json");
        fs::create_dir(next.path().join("next.config.directory")).unwrap();
        assert_matches_targeted(next.path(), "Next.js", &["package.json"]);

        let directory_marker = TempDir::new().unwrap();
        fs::create_dir(directory_marker.path().join("Cargo.toml")).unwrap();
        assert_matches_targeted(directory_marker.path(), "Unknown", &[]);
    }

    #[test]
    fn snapshot_matches_targeted_at_limit_and_falls_back_above_limit() {
        let temp = TempDir::new().unwrap();
        write_marker(temp.path(), "Cargo.toml");
        for index in 1..PROJECT_ROOT_SNAPSHOT_LIMIT {
            write_marker(temp.path(), &format!("source_{index:02}.rs"));
        }

        assert!(ProjectRootSnapshot::read(temp.path()).is_some());
        assert_matches_targeted(temp.path(), "Rust", &["Cargo.toml"]);

        write_marker(temp.path(), "source_64.rs");
        assert!(ProjectRootSnapshot::read(temp.path()).is_none());
        assert_matches_targeted(temp.path(), "Rust", &["Cargo.toml"]);
    }

    #[test]
    fn snapshot_falls_back_for_missing_root() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("missing");

        assert!(ProjectRootSnapshot::read(&missing).is_none());
        assert_matches_targeted(&missing, "Unknown", &[]);
    }

    #[test]
    fn snapshot_preserves_invalid_package_json_behavior() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("package.json"), [0xff, 0xfe]).unwrap();

        assert_matches_targeted(temp.path(), "Node.js", &["package.json"]);
    }

    #[cfg(unix)]
    #[test]
    fn snapshot_does_not_lossily_match_non_utf8_names() {
        use std::collections::HashSet;
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let name = OsString::from_vec(b"next.config.\xff".to_vec());
        let snapshot = ProjectRootSnapshot {
            entry_names: vec![name.clone()],
            file_names: HashSet::from([name]),
        };

        assert_eq!(snapshot.entry_names.len(), 1);
        assert!(!snapshot.has_file("next.config..csproj"));
        assert!(!snapshot.has_prefix("next.config."));
        assert!(!snapshot.has_extension("csproj"));
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn targeted_detector_follows_file_symlinks_but_not_broken_symlinks() {
        let valid = TempDir::new().unwrap();
        write_marker(valid.path(), "actual-package.json");
        symlink_file(
            valid.path().join("actual-package.json"),
            valid.path().join("package.json"),
        );
        assert_matches_targeted(valid.path(), "Node.js", &["package.json"]);

        let broken = TempDir::new().unwrap();
        symlink_file(
            broken.path().join("missing-package.json"),
            broken.path().join("package.json"),
        );
        assert_matches_targeted(broken.path(), "Unknown", &[]);
    }

    #[cfg(unix)]
    fn symlink_file(target: impl AsRef<Path>, link: impl AsRef<Path>) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }

    #[cfg(windows)]
    fn symlink_file(target: impl AsRef<Path>, link: impl AsRef<Path>) {
        std::os::windows::fs::symlink_file(target, link).unwrap();
    }

    #[test]
    fn candidate_prefilter_includes_global_app_cache_names() {
        for name in [
            "ms-playwright",
            "com.microsoft.VSCode.ShipIt",
            "Chrome",
            "GoogleUpdater",
            "LarkInternational",
            "com.google.Chrome.code_sign_clone",
            "remem-dry-run-123",
            "downloads",
            ".downloadIntermediates",
            "build-cache",
            "hosted",
            "git",
            "videos",
            "OptGuideOnDeviceModel",
            "update",
            "MapTiles",
            "MediaCache",
            "com.apple.mediaanalysisd",
            "mod",
            "compact_index",
            "_npx",
            "_logs",
            "_prebuilds",
            "torch_compile_cache",
            "whisper",
            "llama.cpp",
            "Code Cache",
            "DawnGraphiteCache",
            "publisher.tool-1.2.0",
            "1.2.3",
        ] {
            assert!(is_candidate_name(name), "{name} should pass prefilter");
        }

        assert!(
            !is_candidate_name(".ShipIt"),
            "bare .ShipIt must not pass the dynamic prefilter"
        );
    }
}
