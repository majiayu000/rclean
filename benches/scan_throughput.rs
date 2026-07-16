//! End-to-end scan throughput bench, invoked through the release binary.
//!
//! Why spawn the binary instead of calling `scan::scan` directly?
//! rclean-cli is a bin crate (no `src/lib.rs`), so bench crates can't
//! import its modules. Re-exposing modules through a lib would be a
//! large public-API restructure; bench-via-process keeps the build
//! topology simple and measures wall-time the way users actually feel
//! it (including the ~50ms process-startup overhead).
//!
//! Run:
//!   cargo bench --bench scan_throughput
//!
//! The fixtures cover the two scan shapes from issue #111:
//!
//! - many small projects, each with a tiny `node_modules`
//! - one Rust project with a wider `target` tree
//!
//! Candidate count + total reclaimable bytes are deterministic; the
//! only varying inputs are filesystem cache state and clock jitter,
//! which Criterion samples over.

use std::fs;
use std::path::Path;
use std::process::Command;

use criterion::{Criterion, criterion_group, criterion_main};
use tempfile::TempDir;

/// Number of mini projects in the synthetic fixture. Chosen so a scan
/// clears Criterion's noise floor while keeping the bench cheap.
const SMALL_PROJECT_COUNT: usize = 100;
/// File size per dummy artifact blob. 4 KiB stays under macOS's 16 KiB
/// block boundary so allocated_size doesn't dominate.
const BLOB_SIZE: u64 = 4 * 1024;
const HUGE_DIR_COUNT: usize = 64;
const HUGE_FILES_PER_DIR: usize = 32;
const WIDE_PROJECT_COUNT: usize = 20;
const WIDE_SOURCE_FILES_PER_PROJECT: usize = 500;

fn write_sized_file(path: &Path, bytes: u64) {
    let file = fs::File::create(path).unwrap();
    file.set_len(bytes).unwrap();
}

fn build_many_small_fixture(root: &Path) {
    for i in 0..SMALL_PROJECT_COUNT {
        let project = root.join(format!("project_{i:03}"));
        fs::create_dir(&project).unwrap();
        // Node marker.
        fs::write(project.join("package.json"), b"{}").unwrap();
        // Source file so project_bytes > 0.
        fs::write(project.join("index.js"), vec![b'a'; 128]).unwrap();
        // Candidate.
        let nm = project.join("node_modules");
        fs::create_dir(&nm).unwrap();
        write_sized_file(&nm.join("blob"), BLOB_SIZE);
    }
}

fn build_one_huge_fixture(root: &Path) {
    let project = root.join("huge_rust");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("Cargo.toml"),
        b"[package]\nname = \"huge_rust\"\n",
    )
    .unwrap();
    fs::write(project.join("main.rs"), b"fn main() {}\n").unwrap();

    let target = project.join("target");
    for dir_index in 0..HUGE_DIR_COUNT {
        let dir = target
            .join("debug")
            .join("deps")
            .join(format!("crate_{dir_index:03}"));
        fs::create_dir_all(&dir).unwrap();
        for file_index in 0..HUGE_FILES_PER_DIR {
            write_sized_file(&dir.join(format!("artifact_{file_index:03}.o")), BLOB_SIZE);
        }
    }
}

fn build_many_wide_source_fixture(root: &Path) {
    for project_index in 0..WIDE_PROJECT_COUNT {
        let project = root.join(format!("wide_{project_index:03}"));
        let source = project.join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(project.join("package.json"), b"{}").unwrap();

        for file_index in 0..WIDE_SOURCE_FILES_PER_PROJECT {
            fs::write(source.join(format!("source_{file_index:04}.js")), b"source").unwrap();
        }

        let node_modules = project.join("node_modules");
        fs::create_dir(&node_modules).unwrap();
        write_sized_file(&node_modules.join("blob"), BLOB_SIZE);
    }
}

fn run_scan(rclean: &str, root: &Path) {
    let output = Command::new(rclean)
        .args(["scan", root.to_str().unwrap(), "--json", "--min-size", "0"])
        .output()
        .expect("rclean binary should be runnable from CARGO_BIN_EXE_rclean");
    assert!(
        output.status.success(),
        "scan exited non-zero: stderr={}",
        String::from_utf8_lossy(&output.stderr),
    );
}

fn bench_scan_throughput(c: &mut Criterion) {
    let many_small = TempDir::new().unwrap();
    build_many_small_fixture(many_small.path());
    let one_huge = TempDir::new().unwrap();
    build_one_huge_fixture(one_huge.path());
    let many_wide = TempDir::new().unwrap();
    build_many_wide_source_fixture(many_wide.path());

    // Compiled by Cargo when `cargo bench` runs; points at the
    // release-mode rclean binary for this workspace.
    let rclean = env!("CARGO_BIN_EXE_rclean");

    let mut group = c.benchmark_group("scan");
    // Process startup overhead is part of this bench because it tracks
    // what users feel when they invoke the CLI on a workspace.
    group.sample_size(20);
    group.bench_function("many_small_projects_json", |b| {
        b.iter(|| {
            run_scan(rclean, many_small.path());
        });
    });
    group.bench_function("one_huge_candidate_json", |b| {
        b.iter(|| {
            run_scan(rclean, one_huge.path());
        });
    });
    group.bench_function("many_wide_source_projects_json", |b| {
        b.iter(|| {
            run_scan(rclean, many_wide.path());
        });
    });
    group.finish();
}

criterion_group!(benches, bench_scan_throughput);
criterion_main!(benches);
