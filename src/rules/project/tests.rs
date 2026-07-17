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
