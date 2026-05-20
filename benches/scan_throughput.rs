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
//! The fixture is a synthetic monorepo of 50 mini "projects", each
//! with a Node-style marker and a populated `node_modules`. The
//! candidate count + total reclaimable bytes are deterministic; the
//! only varying inputs are filesystem cache state and clock jitter,
//! which Criterion samples over.

use std::path::Path;
use std::process::Command;

use criterion::{Criterion, criterion_group, criterion_main};
use tempfile::TempDir;

/// Number of mini projects in the synthetic fixture. Chosen so a single
/// scan takes longer than Criterion's noise floor (~50ms) but short
/// enough to keep the bench iteration count high (~50 runs per group).
const PROJECT_COUNT: usize = 50;
/// File size per dummy `node_modules` blob. 4 KiB stays under macOS's
/// 16 KiB block boundary so allocated_size doesn't dominate.
const BLOB_SIZE: usize = 4 * 1024;

fn build_fixture(root: &Path) {
    for i in 0..PROJECT_COUNT {
        let project = root.join(format!("project_{i:03}"));
        std::fs::create_dir(&project).unwrap();
        // Node marker.
        std::fs::write(project.join("package.json"), b"{}").unwrap();
        // Source file so project_bytes > 0.
        std::fs::write(project.join("index.js"), vec![b'a'; 128]).unwrap();
        // Candidate.
        let nm = project.join("node_modules");
        std::fs::create_dir(&nm).unwrap();
        std::fs::write(nm.join("blob"), vec![b'x'; BLOB_SIZE]).unwrap();
    }
}

fn bench_scan_throughput(c: &mut Criterion) {
    let temp = TempDir::new().unwrap();
    build_fixture(temp.path());

    // Compiled by Cargo when `cargo bench` runs; points at the
    // release-mode rclean binary for this workspace.
    let rclean = env!("CARGO_BIN_EXE_rclean");

    let mut group = c.benchmark_group("scan");
    // 50 projects × ~4 KiB each = 200 KiB of "reclaimable" payload.
    // Process startup overhead dominates at this size, which is the
    // point: this bench tracks "what does a user feel when they hit
    // rclean on a small workspace". A larger fixture is owned by
    // docs/perf/v0.1.5.md, not this in-repo bench.
    group.sample_size(20);
    group.bench_function("synthetic_50_projects_json", |b| {
        b.iter(|| {
            let output = Command::new(rclean)
                .args([
                    "scan",
                    temp.path().to_str().unwrap(),
                    "--json",
                    "--min-size",
                    "0",
                ])
                .output()
                .expect("rclean binary should be runnable from CARGO_BIN_EXE_rclean");
            assert!(
                output.status.success(),
                "scan exited non-zero: stderr={}",
                String::from_utf8_lossy(&output.stderr),
            );
        });
    });
    group.finish();
}

criterion_group!(benches, bench_scan_throughput);
criterion_main!(benches);
