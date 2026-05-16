use std::fs;
use std::path::{Path, PathBuf};

pub fn has_marker(dir: &Path, marker: &str) -> bool {
    dir.join(marker).is_file()
}

pub fn has_prefixed_marker(dir: &Path, prefix: &str) -> bool {
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

pub fn has_any_project_marker(dir: &Path) -> bool {
    fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(super::is_project_marker_name)
        })
    })
}

pub fn is_node_project(dir: &Path) -> bool {
    has_marker(dir, "package.json")
}

pub fn is_python_project(dir: &Path) -> bool {
    ["pyproject.toml", "requirements.txt", "setup.py", "Pipfile"]
        .iter()
        .any(|marker| has_marker(dir, marker))
}

pub fn is_gradle_project(dir: &Path) -> bool {
    has_marker(dir, "build.gradle") || has_marker(dir, "build.gradle.kts")
}

pub fn is_ruby_project(dir: &Path) -> bool {
    has_marker(dir, "Gemfile")
}

pub fn is_dotnet_project(dir: &Path) -> bool {
    has_marker_with_extension(dir, "sln")
        || has_marker_with_extension(dir, "csproj")
        || has_marker_with_extension(dir, "fsproj")
}

pub fn has_marker_with_extension(dir: &Path, extension: &str) -> bool {
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

pub fn package_mentions(dir: &Path, dep: &str) -> bool {
    let Ok(raw) = fs::read_to_string(dir.join("package.json")) else {
        return false;
    };
    let needle = format!("\"{dep}\"");
    raw.contains(&needle)
}

pub fn is_virtualenv(path: &Path) -> bool {
    path.join("pyvenv.cfg").is_file()
        || path.join("bin").join("activate").is_file()
        || path.join("Scripts").join("activate").is_file()
}

pub fn is_shared_cargo_target(project_dir: &Path, candidate: &Path) -> bool {
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
