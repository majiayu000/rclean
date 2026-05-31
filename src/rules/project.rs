use std::path::Path;

use super::markers::{
    has_marker, has_prefixed_marker, is_dotnet_project, is_python_project, is_ruby_project,
    package_mentions,
};
use super::node_global::is_pnpm_store_version_name;

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
            | "go-build"
            | "store"
            | "_cacache"
            | "Yarn"
            | "pip"
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
    ) || is_pnpm_store_version_name(name)
        || is_shipit_candidate_name(name)
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
            | "node.npm_cacache"
            | "node.yarn_cache"
            | "node.pnpm_store"
            | "js.deno_cache"
            | "pip.cache"
            | "python.uv_cache"
            | "python.poetry_cache"
            | "python.pipx_cache"
            | "gradle.caches"
            | "maven.local_repo"
            | "xcode.simulators"
            | "go.module_download_cache"
            | "go.build_cache"
            | "bun.cache"
            | "pre_commit.cache"
            | "playwright.browsers"
            | "app.shipit_caches"
            | "chrome.cache"
            | "chrome.google_updater"
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
    let mut markers = Vec::new();

    for marker in [
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
    ] {
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
    use super::is_candidate_name;

    #[test]
    fn candidate_prefilter_includes_global_app_cache_names() {
        for name in [
            "ms-playwright",
            "com.microsoft.VSCode.ShipIt",
            "Chrome",
            "GoogleUpdater",
        ] {
            assert!(is_candidate_name(name), "{name} should pass prefilter");
        }

        assert!(
            !is_candidate_name(".ShipIt"),
            "bare .ShipIt must not pass the dynamic prefilter"
        );
    }
}
