use std::path::{Path, PathBuf};

use crate::model::{CandidateDraft, Category};

mod cargo_global;
mod catalog;
mod dotnet;
mod flutter;
mod generic;
mod go;
mod gradle;
mod ios;
mod jvm;
mod markers;
mod node;
mod node_global;
mod pip;
mod python;
mod ruby;
mod rust;
mod xcode;

use markers::{
    has_marker, has_prefixed_marker, is_dotnet_project, is_python_project, is_ruby_project,
    is_shared_cargo_target, package_mentions,
};

#[derive(Debug, Clone)]
pub struct RuleInfo {
    pub rule_id: &'static str,
    pub category: Category,
    pub candidate: &'static str,
    pub restore_hint: &'static str,
}

pub fn rule_catalog() -> Vec<RuleInfo> {
    catalog::RULES.to_vec()
}

pub fn classify_candidate(project_dir: &Path, name: &str, path: PathBuf) -> Option<CandidateDraft> {
    let path_ref = path.as_path();
    // Order matches v0.1.0's match-arm priority. The ambiguous `build/`
    // directory name belongs to jvm (Gradle) and flutter (Dart) before
    // node — under a mixed Gradle+Node project, `build/` is a Gradle
    // output (Safety::Safe) and must not be reclassified as a Node
    // caution candidate. Same logic for `target/`: rust > jvm.
    let draft = rust::classify(project_dir, name, path_ref)
        .or_else(|| jvm::classify(project_dir, name, path_ref))
        .or_else(|| flutter::classify(project_dir, name, path_ref))
        .or_else(|| node::classify(project_dir, name, path_ref))
        .or_else(|| python::classify(project_dir, name, path_ref))
        .or_else(|| dotnet::classify(project_dir, name, path_ref))
        .or_else(|| ruby::classify(project_dir, name, path_ref))
        .or_else(|| go::classify(project_dir, name, path_ref))
        .or_else(|| ios::classify(project_dir, name, path_ref))
        .or_else(|| xcode::classify(project_dir, name, path_ref))
        .or_else(|| cargo_global::classify(project_dir, name, path_ref))
        .or_else(|| node_global::classify(project_dir, name, path_ref))
        .or_else(|| pip::classify(project_dir, name, path_ref))
        .or_else(|| gradle::classify(project_dir, name, path_ref))
        .or_else(|| generic::classify(project_dir, name, path_ref));

    draft.map(|mut draft| {
        if is_shared_cargo_target(project_dir, &draft.path) {
            draft.safety = crate::model::Safety::Blocked;
            draft
                .warnings
                .push("shared Cargo target directory detected".to_string());
        }
        draft
    })
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
            | "db"
            | "_cacache"
            | "Yarn"
            | "pip"
            | "caches"
    )
}

/// Returns true for rule ids whose classifier intentionally targets
/// paths inside the user runtime/system tree (e.g. `~/Library/...`).
/// `apply_path_safety` skips the generic runtime-path block for these
/// — the classifier already established that the path is a
/// rebuildable cache, not user data.
pub fn is_global_rule(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "xcode.derived_data"
            | "cargo.registry_cache"
            | "cargo.git_db"
            | "node.npm_cacache"
            | "node.yarn_cache"
            | "pip.cache"
            | "gradle.caches"
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
