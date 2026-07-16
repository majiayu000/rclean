# GH287 Tasks

## Linked Artifacts

- Issue: `#287`
- Product spec: `specs/GH287/product.md`
- Tech spec: `specs/GH287/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH287 Spec PR to merge.

## Implementation Tasks

### SP287-T1 — Extract unchanged targeted detector and build reference tests

- Owner: `implementation`
- Dependencies: merged GH287 Spec PR; latest `origin/main`
- Covers: B-003, B-004, B-005, B-006, B-007, B-009
- Change: move the current `detect_project_kind` body into a private targeted helper without behavior
  changes; add table-driven fixtures for every kind, marker ordering, mixed priority and current
  file/directory/symlink/prefix/extension semantics.
- Done when: public detector and targeted helper return identical `(kind, markers)` before the fast
  path is enabled, and tests can use targeted as the fallback oracle.
- Verify:
  - `cargo test rules::project::tests`
  - `git diff` confirms no `markers.rs` or classifier change

### SP287-T2 — Add bounded marker snapshot fast path

- Owner: `implementation`
- Dependencies: SP287-T1
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009
- Change: add raw-name snapshot reader, separate entry/file predicates, 64-entry cap, full fallback,
  one-read package content, and snapshot detector with unchanged priority/order.
- Done when: <=64 roots use snapshot results; 65+/missing/error roots use the unchanged targeted
  helper; all semantic reference tests pass.
- Verify:
  - `cargo test rules::project::tests`
  - focused boundary, symlink, non-UTF-8 and mixed-priority assertions

### SP287-T3 — Add durable 1,000-project marker benchmark

- Owner: `implementation`
- Dependencies: SP287-T2
- Covers: B-001, B-011
- Change: add a marker-heavy 1,000-small-project fixture/benchmark outside timed setup and preserve
  the existing three benchmark shapes.
- Done when: Criterion reports all four shapes without adding flaky timing assertions.
- Verify:
  - `cargo bench --bench scan_throughput -- --noplot`

## Verification And Handoff Tasks

### SP287-T4 — Prove report equivalence and performance

- Owner: `verification`
- Dependencies: SP287-T1, SP287-T2, SP287-T3
- Covers: B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011
- Done when: normalized JSON diff is empty; 15-run 1,000-project median improves >=15%; each
  existing Criterion point estimate regresses <=10%.
- Verify:
  - normalized JSON comparison removing only `scannedAt`
  - same-session before/after timing table
  - before/after Criterion table for 100-small, one-huge and many-wide

### SP287-T5 — Full gate, VibeGuard and SpecRail audit

- Owner: `verification`
- Dependencies: SP287-T4
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011
- Done when: scope is exactly two planned files; full stable/release/MSRV/VibeGuard and current-head
  PR gates pass; spec-vs-implementation has no missing invariant or private-advisory leakage.
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

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011}`
- Missing invariants: `none`

## Handoff Notes

- Do not change `markers.rs`, classifiers, safety, delete, ActionPlan or private advisory artifacts.
- Preserve targeted detector behavior as the fallback oracle; no opportunistic correctness cleanup.
- Do not use lossy string conversion or an unbounded root inventory.
- Implementation starts from merged Spec PR on latest `origin/main`.
- Merge only with fresh current-head gates under standing authorization; never force push.
