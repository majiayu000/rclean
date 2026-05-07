use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{CandidateDraft, Category, Safety};

mod catalog;

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
    let mut reasons = Vec::new();
    let mut warnings = Vec::new();

    let mut draft = match name {
        "node_modules" if has_marker(project_dir, "package.json") => {
            reasons.push("package.json marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "node.node_modules".to_string(),
                category: Category::Deps,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Run npm install, pnpm install, yarn install, or bun install"
                    .to_string(),
            }
        }
        ".next" if is_node_project(project_dir) => {
            reasons.push("Node project marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "node.next".to_string(),
                category: Category::Build,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Run the Next.js build or dev command".to_string(),
            }
        }
        ".turbo" if is_node_project(project_dir) => {
            reasons.push("Node project marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "node.turbo".to_string(),
                category: Category::Cache,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Rebuilt by Turborepo".to_string(),
            }
        }
        ".vite" if is_node_project(project_dir) => {
            reasons.push("Node project marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "node.vite".to_string(),
                category: Category::Cache,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Rebuilt by Vite".to_string(),
            }
        }
        ".parcel-cache" if is_node_project(project_dir) => {
            reasons.push("Node project marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "node.parcel".to_string(),
                category: Category::Cache,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Rebuilt by Parcel".to_string(),
            }
        }
        ".venv" if is_python_project(project_dir) && is_virtualenv(&path) => {
            reasons.push("Python marker and virtualenv marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "python.venv_dot".to_string(),
                category: Category::Deps,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Recreate the Python environment".to_string(),
            }
        }
        "venv" if is_python_project(project_dir) => {
            if is_virtualenv(&path) {
                reasons.push("Python marker and virtualenv marker found".to_string());
                CandidateDraft {
                    path,
                    name: name.to_string(),
                    rule_id: "python.venv_plain".to_string(),
                    category: Category::Deps,
                    safety: Safety::Safe,
                    reasons,
                    warnings,
                    restore_hint: "Recreate the Python environment".to_string(),
                }
            } else {
                warnings.push("plain venv directory has no virtualenv marker".to_string());
                CandidateDraft {
                    path,
                    name: name.to_string(),
                    rule_id: "python.venv_plain".to_string(),
                    category: Category::Deps,
                    safety: Safety::Blocked,
                    reasons,
                    warnings,
                    restore_hint: "Not deleted because this is not validated as a virtualenv"
                        .to_string(),
                }
            }
        }
        "__pycache__" if is_python_project(project_dir) => {
            reasons.push("Python project marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "python.pycache".to_string(),
                category: Category::Cache,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Recreated by Python".to_string(),
            }
        }
        ".pytest_cache" | ".mypy_cache" | ".ruff_cache" | ".tox"
            if is_python_project(project_dir) =>
        {
            reasons.push("Python project marker found".to_string());
            let (rule_id, hint, safety) = match name {
                ".pytest_cache" => ("python.pytest", "Recreated by pytest", Safety::Safe),
                ".mypy_cache" => ("python.mypy", "Recreated by mypy", Safety::Safe),
                ".ruff_cache" => ("python.ruff", "Recreated by ruff", Safety::Safe),
                _ => {
                    warnings.push(".tox may contain expensive local test environments".to_string());
                    ("python.tox", "Recreated by tox", Safety::Caution)
                }
            };
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: rule_id.to_string(),
                category: Category::Cache,
                safety,
                reasons,
                warnings,
                restore_hint: hint.to_string(),
            }
        }
        "target" if has_marker(project_dir, "Cargo.toml") => {
            reasons.push("Cargo.toml marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "rust.target".to_string(),
                category: Category::Build,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Run cargo build or cargo test".to_string(),
            }
        }
        "target" if has_marker(project_dir, "pom.xml") => {
            reasons.push("pom.xml marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "java.maven_target".to_string(),
                category: Category::Build,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Run Maven build".to_string(),
            }
        }
        "vendor" if has_marker(project_dir, "go.mod") => {
            reasons.push("go.mod marker found".to_string());
            warnings.push("vendor may contain intentionally checked-in dependencies".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "go.vendor".to_string(),
                category: Category::Deps,
                safety: Safety::Caution,
                reasons,
                warnings,
                restore_hint: "Run go mod vendor".to_string(),
            }
        }
        "Pods" if has_marker(project_dir, "Podfile") => {
            reasons.push("Podfile marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "ios.pods".to_string(),
                category: Category::Deps,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Run pod install".to_string(),
            }
        }
        ".gradle" if is_gradle_project(project_dir) => {
            reasons.push("Gradle marker found".to_string());
            warnings.push(".gradle may contain useful local Gradle state".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "java.gradle_cache_local".to_string(),
                category: Category::Cache,
                safety: Safety::Caution,
                reasons,
                warnings,
                restore_hint: "Rebuilt by Gradle".to_string(),
            }
        }
        ".dart_tool" if has_marker(project_dir, "pubspec.yaml") => {
            reasons.push("pubspec.yaml marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "dart.tool".to_string(),
                category: Category::Cache,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Run flutter pub get or dart pub get".to_string(),
            }
        }
        "bin" | "obj" if is_dotnet_project(project_dir) => {
            reasons.push(".NET project marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: format!("dotnet.{name}"),
                category: Category::Build,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Run dotnet build".to_string(),
            }
        }
        ".bundle" if is_ruby_project(project_dir) => {
            reasons.push("Ruby project marker found".to_string());
            warnings.push(".bundle can contain local Bundler configuration".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "ruby.bundle".to_string(),
                category: Category::Cache,
                safety: Safety::Caution,
                reasons,
                warnings,
                restore_hint: "Run bundle install".to_string(),
            }
        }
        "vendor" if is_ruby_project(project_dir) && path.join("bundle").is_dir() => {
            reasons.push("Ruby project marker and vendor/bundle found".to_string());
            warnings.push("vendor/bundle may contain intentionally vendored gems".to_string());
            CandidateDraft {
                path: path.join("bundle"),
                name: "vendor/bundle".to_string(),
                rule_id: "ruby.vendor_bundle".to_string(),
                category: Category::Deps,
                safety: Safety::Caution,
                reasons,
                warnings,
                restore_hint: "Run bundle install".to_string(),
            }
        }
        "coverage" if has_any_project_marker(project_dir) => {
            reasons.push("project marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "generic.coverage".to_string(),
                category: Category::Test,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Re-run the test suite".to_string(),
            }
        }
        "build" if is_gradle_project(project_dir) => {
            reasons.push("Gradle marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "java.gradle_build".to_string(),
                category: Category::Build,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Run Gradle build".to_string(),
            }
        }
        "build" if has_marker(project_dir, "pubspec.yaml") => {
            reasons.push("pubspec.yaml marker found".to_string());
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: "dart.build".to_string(),
                category: Category::Build,
                safety: Safety::Safe,
                reasons,
                warnings,
                restore_hint: "Run flutter build or dart build".to_string(),
            }
        }
        "build" | "dist" | "out" if is_node_project(project_dir) => {
            reasons.push("Node project marker found".to_string());
            warnings.push(format!("{name} is generic and may contain user outputs"));
            CandidateDraft {
                path,
                name: name.to_string(),
                rule_id: format!("node.{name}"),
                category: Category::Build,
                safety: Safety::Caution,
                reasons,
                warnings,
                restore_hint: "Re-run the project build".to_string(),
            }
        }
        _ => return None,
    };

    if is_shared_cargo_target(project_dir, &draft.path) {
        draft.safety = Safety::Blocked;
        draft
            .warnings
            .push("shared Cargo target directory detected".to_string());
    }

    Some(draft)
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

fn has_marker(dir: &Path, marker: &str) -> bool {
    dir.join(marker).is_file()
}

fn has_prefixed_marker(dir: &Path, prefix: &str) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(prefix))
    })
}

fn has_any_project_marker(dir: &Path) -> bool {
    fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(is_project_marker_name)
        })
    })
}

fn is_node_project(dir: &Path) -> bool {
    has_marker(dir, "package.json")
}

fn is_python_project(dir: &Path) -> bool {
    ["pyproject.toml", "requirements.txt", "setup.py", "Pipfile"]
        .iter()
        .any(|marker| has_marker(dir, marker))
}

fn is_gradle_project(dir: &Path) -> bool {
    has_marker(dir, "build.gradle") || has_marker(dir, "build.gradle.kts")
}

fn is_ruby_project(dir: &Path) -> bool {
    has_marker(dir, "Gemfile")
}

fn is_dotnet_project(dir: &Path) -> bool {
    has_marker_with_extension(dir, "sln")
        || has_marker_with_extension(dir, "csproj")
        || has_marker_with_extension(dir, "fsproj")
}

fn has_marker_with_extension(dir: &Path, extension: &str) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        entry
            .path()
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(extension))
    })
}

fn package_mentions(dir: &Path, dep: &str) -> bool {
    let Ok(raw) = fs::read_to_string(dir.join("package.json")) else {
        return false;
    };
    let needle = format!("\"{dep}\"");
    raw.contains(&needle)
}

fn is_virtualenv(path: &Path) -> bool {
    path.join("pyvenv.cfg").is_file()
        || path.join("bin").join("activate").is_file()
        || path.join("Scripts").join("activate").is_file()
}

fn is_shared_cargo_target(project_dir: &Path, candidate: &Path) -> bool {
    if candidate.file_name().and_then(|name| name.to_str()) != Some("target") {
        return false;
    }

    if let Ok(raw) = std::env::var("CARGO_TARGET_DIR")
        && !raw.trim().is_empty()
    {
        let target = PathBuf::from(raw);
        if same_path(candidate, &target) {
            return true;
        }
    }

    for config in [
        project_dir.join(".cargo").join("config.toml"),
        project_dir.join(".cargo").join("config"),
    ] {
        let Ok(raw) = fs::read_to_string(config) else {
            continue;
        };
        for line in raw.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("target-dir") {
                continue;
            }
            if trimmed.contains('/') && !trimmed.contains("\"target\"") {
                return true;
            }
        }
    }

    false
}

fn same_path(a: &Path, b: &Path) -> bool {
    let Ok(a) = a.canonicalize() else {
        return false;
    };
    let b = if b.is_absolute() {
        b.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(b),
            Err(_) => return false,
        }
    };
    b.canonicalize().is_ok_and(|b| a == b)
}
