# GH284 Tasks

## Linked Artifacts

- Issue: `#284`
- Product spec: `specs/GH284/product.md`
- Tech spec: `specs/GH284/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH284 Spec PR to merge.

## Implementation Tasks

### SP284-T1 — Add ordered activity batch helper and focused tests

- Owner: `implementation`
- Dependencies: merged GH284 Spec PR; latest `origin/main`
- Covers: B-001, B-002, B-003, B-004, B-005, B-006
- Change: add empty/single/multi branches around the unchanged `project_activity`; use existing
  Rayon indexed `par_iter` only for multi-project input; add deterministic reference-equivalence,
  order and fallback tests.
- Done when: every input retains one ordered result, single input avoids parallel dispatch, and
  nested/depth/pruned/symlink fixtures match serial `project_activity` calls.
- Verify:
  - `cargo test scan::project::tests`
  - source review confirms no custom pool/thread and unchanged traversal body

### SP284-T2 — Feed precomputed activity into serial report materialization

- Owner: `implementation`
- Dependencies: SP284-T1
- Covers: B-004, B-006, B-007, B-008
- Change: make `build_project_report` consume `activity_time`; compute the ordered vector after
  sorting `project_dirs`, then zip into the existing serial loop.
- Done when: report build performs no activity traversal, no project is dropped, and the supplied
  time consistently drives filter/output/staleness/risk fields.
- Verify:
  - `cargo test scan::project::tests`
  - `cargo test scan::tests::scan_sizes_one_large_candidate_and_many_small_projects_deterministically`
  - `cargo test scan::tests::dirty_git_marks_candidate_caution`

### SP284-T3 — Add durable activity throughput shape

- Owner: `implementation`
- Dependencies: SP284-T2
- Covers: B-003, B-010
- Change: extend `benches/scan_throughput.rs` with a bounded multi-project, wide non-candidate source
  fixture; build it outside timed closures and keep both existing benchmark shapes unchanged.
- Done when: Criterion reports all three shapes without changing report assertions or adding a
  wall-clock assertion to CI.
- Verify:
  - `cargo bench --bench scan_throughput -- --noplot`

## Verification And Handoff Tasks

### SP284-T4 — Prove report equivalence and same-session performance

- Owner: `verification`
- Dependencies: SP284-T1, SP284-T2, SP284-T3
- Covers: B-004, B-005, B-006, B-007, B-008, B-009, B-010
- Done when: fixed-mtime before/after normalized JSON diff is empty; five warmed 40,040-file runs
  show >=20% median improvement; 100-small Criterion after is <=110% of before.
- Verify:
  - normalized release JSON comparison removing only top-level `scannedAt`
  - same-session release timing table
  - before/after Criterion point estimates

### SP284-T5 — Full gate, VibeGuard and SpecRail audit

- Owner: `verification`
- Dependencies: SP284-T4
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010
- Done when: scope is exactly the three-file planned manifest; full/default/release/MSRV/VibeGuard
  and current-head PR gates pass; spec-vs-implementation has no missing invariant.
- Verify:
  - `git diff --check`
  - `git diff --name-only origin/main...HEAD`
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - all installed VibeGuard Rust guards

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010}`
- Missing invariants: `none`

## Handoff Notes

- Do not change the `project_activity` traversal implementation or explain single-path behavior.
- Do not parallelize Git, sizing, warnings or report construction.
- Do not add dependencies or modify any file outside the planned three-file manifest.
- Implementation starts from the merged Spec PR on latest `origin/main`.
- Merge only with fresh current-head gates under the standing authorization; never force push.
