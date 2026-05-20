use std::fs;
use std::io::Write;
use std::process::Command;
use std::time::{Duration, SystemTime};

use tempfile::TempDir;

use crate::model::{GitInfo, ProjectReport, Safety};

use super::*;

fn options() -> ScanOptions {
    ScanOptions {
        max_depth: 6,
        min_size: 0,
        older_than: None,
        categories: None,
        rule_ids: None,
        include_blocked: true,
        verbose: false,
        ignore_globs: Vec::new(),
    }
}

#[test]
fn detects_root_node_project() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();
    fs::create_dir(temp.path().join("node_modules")).unwrap();
    fs::write(temp.path().join("node_modules").join("x"), "abc").unwrap();

    let report = scan(&[temp.path().to_path_buf()], &options()).unwrap();

    assert_eq!(report.summary.candidates, 1);
    assert_eq!(
        report.projects[0].candidates[0].rule_id,
        "node.node_modules"
    );
    assert_eq!(report.projects[0].total_bytes, 3);
    assert_eq!(report.projects[0].project_bytes, 5);
    assert_eq!(report.projects[0].artifact_percent, 60.0);
}

#[test]
fn blocks_plain_python_venv_without_marker() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("pyproject.toml"), "[project]\n").unwrap();
    fs::create_dir(temp.path().join("venv")).unwrap();

    let report = scan(&[temp.path().to_path_buf()], &options()).unwrap();

    assert_eq!(report.projects[0].candidates[0].safety, Safety::Blocked);
}

#[test]
fn generic_build_without_marker_is_ignored() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("build")).unwrap();

    let report = scan(&[temp.path().to_path_buf()], &options()).unwrap();

    assert_eq!(report.summary.candidates, 0);
}

#[test]
fn symlink_candidate_is_blocked() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();
    let real = temp.path().join("real_modules");
    fs::create_dir(&real).unwrap();
    let link = temp.path().join("node_modules");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real, &link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&real, &link).unwrap();

    let report = scan(&[temp.path().to_path_buf()], &options()).unwrap();

    assert_eq!(report.projects[0].candidates[0].safety, Safety::Blocked);
}

#[test]
fn detects_gradle_dart_dotnet_and_ruby_rules() {
    let temp = TempDir::new().unwrap();

    let gradle = temp.path().join("gradle");
    fs::create_dir(&gradle).unwrap();
    fs::write(gradle.join("build.gradle"), "plugins {}\n").unwrap();
    fs::create_dir(gradle.join("build")).unwrap();

    let dart = temp.path().join("dart");
    fs::create_dir(&dart).unwrap();
    fs::write(dart.join("pubspec.yaml"), "name: app\n").unwrap();
    fs::create_dir(dart.join(".dart_tool")).unwrap();

    let dotnet = temp.path().join("dotnet");
    fs::create_dir(&dotnet).unwrap();
    fs::write(dotnet.join("app.csproj"), "<Project />\n").unwrap();
    fs::create_dir(dotnet.join("bin")).unwrap();

    let ruby = temp.path().join("ruby");
    fs::create_dir_all(ruby.join("vendor").join("bundle")).unwrap();
    fs::write(ruby.join("Gemfile"), "source 'https://rubygems.org'\n").unwrap();

    let report = scan(&[temp.path().to_path_buf()], &options()).unwrap();
    let rule_ids = report
        .projects
        .iter()
        .flat_map(|project| project.candidates.iter())
        .map(|candidate| candidate.rule_id.as_str())
        .collect::<Vec<_>>();

    assert!(rule_ids.contains(&"java.gradle_build"));
    assert!(rule_ids.contains(&"dart.tool"));
    assert!(rule_ids.contains(&"dotnet.bin"));
    assert!(rule_ids.contains(&"ruby.vendor_bundle"));
}

#[test]
fn dirty_git_marks_candidate_caution() {
    let temp = TempDir::new().unwrap();
    Command::new("git")
        .arg("-C")
        .arg(temp.path())
        .arg("init")
        .output()
        .unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();
    fs::create_dir(temp.path().join("node_modules")).unwrap();
    let mut file = fs::File::create(temp.path().join("node_modules").join("x")).unwrap();
    writeln!(file, "abc").unwrap();

    let report = scan(&[temp.path().to_path_buf()], &options()).unwrap();

    assert_eq!(report.projects[0].candidates[0].safety, Safety::Caution);
}

#[test]
fn risk_score_is_zero_when_no_markers_trip() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("Cargo.lock"), "[]").unwrap();
    let old = SystemTime::now() - Duration::from_secs(30 * 24 * 60 * 60);
    assert_eq!(compute_risk_score(None, old, temp.path()), 0.0);
}

#[test]
fn risk_score_weights_match_spec() {
    let temp = TempDir::new().unwrap();
    let old = SystemTime::now() - Duration::from_secs(30 * 24 * 60 * 60);
    assert!((compute_risk_score(None, old, temp.path()) - 0.20).abs() < 1e-6);

    fs::write(temp.path().join("Cargo.lock"), "[]").unwrap();
    let recent = SystemTime::now();
    assert!((compute_risk_score(None, recent, temp.path()) - 0.25).abs() < 1e-6);

    let dirty = GitInfo {
        repo_root: temp.path().display().to_string(),
        dirty: true,
    };
    let score = compute_risk_score(Some(&dirty), recent, temp.path());
    assert!((score - 0.65).abs() < 1e-6, "expected 0.65, got {score}");
}

#[test]
fn risk_score_max_is_0_85_until_root_boundary_axis_lands() {
    let temp = TempDir::new().unwrap();
    let recent = SystemTime::now();
    let dirty = GitInfo {
        repo_root: temp.path().display().to_string(),
        dirty: true,
    };
    let score = compute_risk_score(Some(&dirty), recent, temp.path());
    assert!(score <= 1.0, "clamp invariant: score never exceeds 1.0");
    assert!(
        (score - 0.85).abs() < 1e-6,
        "current max is 0.85 (root_boundary deferred); got {score}"
    );
}

#[test]
fn sibling_projects_source_bytes_do_not_cross_contaminate() {
    let temp = TempDir::new().unwrap();

    let frontend = temp.path().join("frontend");
    let backend = temp.path().join("backend");
    fs::create_dir(&frontend).unwrap();
    fs::create_dir(&backend).unwrap();

    let frontend_marker = b"{}";
    let frontend_readme = b"hi\n";
    fs::write(frontend.join("package.json"), frontend_marker).unwrap();
    fs::write(frontend.join("README.md"), frontend_readme).unwrap();
    fs::create_dir(frontend.join("node_modules")).unwrap();
    fs::write(frontend.join("node_modules").join("a"), b"xyz").unwrap();
    let frontend_source_expected = (frontend_marker.len() + frontend_readme.len()) as u64;

    let backend_marker = b"[package]\nname=\"b\"\n";
    let backend_readme = b"hello\n";
    fs::write(backend.join("Cargo.toml"), backend_marker).unwrap();
    fs::write(backend.join("README.md"), backend_readme).unwrap();
    fs::create_dir(backend.join("target")).unwrap();
    fs::write(backend.join("target").join("a"), b"build-output").unwrap();
    let backend_source_expected = (backend_marker.len() + backend_readme.len()) as u64;

    let report = scan(&[temp.path().to_path_buf()], &options()).unwrap();

    let projects: std::collections::HashMap<&str, &ProjectReport> = report
        .projects
        .iter()
        .map(|p| {
            let name = std::path::Path::new(&p.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap();
            (name, p)
        })
        .collect();

    let front = projects.get("frontend").expect("frontend project missing");
    let back = projects.get("backend").expect("backend project missing");

    let front_source = front.project_bytes - front.total_bytes;
    let back_source = back.project_bytes - back.total_bytes;

    assert_eq!(
        front_source, frontend_source_expected,
        "frontend should only count its own files"
    );
    assert_eq!(
        back_source, backend_source_expected,
        "backend should only count its own files"
    );
}

#[test]
fn git_cache_returns_none_for_non_repo() {
    let temp = TempDir::new().unwrap();
    let cache = GitCache::new();

    assert!(cache.info_for(temp.path()).is_none());
    assert!(cache.info_for(temp.path()).is_none());
}

#[test]
fn git_cache_shares_dirty_flag_across_sibling_projects() {
    let temp = TempDir::new().unwrap();
    Command::new("git")
        .arg("-C")
        .arg(temp.path())
        .arg("init")
        .output()
        .unwrap();
    fs::create_dir(temp.path().join("a")).unwrap();
    fs::create_dir(temp.path().join("b")).unwrap();
    fs::write(temp.path().join("dirty.txt"), "x").unwrap();

    let cache = GitCache::new();
    let a = cache.info_for(&temp.path().join("a")).unwrap();
    let b = cache.info_for(&temp.path().join("b")).unwrap();

    assert_eq!(a.repo_root, b.repo_root);
    assert!(a.dirty);
    assert!(b.dirty);
}
