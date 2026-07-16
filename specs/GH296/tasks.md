# GH296 Tasks

## Linked Artifacts

- Issue: `#296`
- Product spec: `specs/GH296/product.md`
- Tech spec: `specs/GH296/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH296 Spec PR to merge.

## Implementation Tasks

### SP296-T1 — Lock the lazy zero/once/reuse contract

- Owner: `implementation`
- Dependencies: merged GH296 Spec PR; latest `origin/main`
- Covers: B-001, B-002, B-003, B-004
- Change: add the minimal private lazy-score helper/test seam and focused counting tests without
  changing `compute_risk_score` or `has_lockfile` behavior.
- Done when: focused tests prove an unused cell performs zero work, first access computes once, and
  later accesses reuse the first value without executing their closures.
- Verify: `cargo test scan::project::tests`

### SP296-T2 — Reuse one risk value per project report

- Owner: `implementation`
- Dependencies: SP296-T1
- Covers: B-001, B-002, B-003, B-004, B-005, B-006
- Change: create a call-local lazy cell before candidate materialization and access it only after the
  existing min-size/safety filter, preserving all loop ordering and fields.
- Done when: zero included candidates do not initialize risk; one or more initialize exactly once;
  existing risk/scan/explain tests and normalized JSON remain unchanged.
- Verify:
  - `cargo test scan::project::tests`
  - `cargo test scan::tests`
  - normalized baseline/implementation JSON diff

### SP296-T3 — Add durable multi-candidate benchmark coverage

- Owner: `implementation`
- Dependencies: SP296-T2
- Covers: B-007, B-008
- Change: add a bounded Node fixture with multiple existing candidates per project and a distinct
  Criterion bench function; keep fixture construction outside timed closures and preserve all
  existing shapes.
- Done when: Criterion reports the new shape and all previous shapes with no timing assertions.
- Verify: `cargo bench --bench scan_throughput -- --noplot`

## Verification And Handoff Tasks

### SP296-T4 — Prove equivalence and material performance improvement

- Owner: `verification`
- Dependencies: SP296-T1, SP296-T2, SP296-T3
- Covers: B-003, B-004, B-005, B-006, B-007, B-008
- Done when: normalized JSON diff is empty; 1,000×8 static probes drop from about 96,000 to about
  12,000; at least 15 interleaved warmed samples per revision show >=15% after-median improvement;
  existing Criterion point estimates regress <=10%.
- Verify:
  - normalized JSON comparison removing only `scannedAt`
  - interleaved same-session release timing table
  - before/after Criterion table

### SP296-T5 — Full stable/MSRV/VibeGuard/SpecRail gate

- Owner: `verification`
- Dependencies: SP296-T4
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009
- Done when: implementation scope is exactly the two planned files; full stable/release/MSRV,
  all installed VibeGuard Rust guards, current-head CI/review-thread/signature/merge-state and
  spec-vs-implementation checks pass.
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

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009}`
- Missing invariants: `none`

## Handoff Notes

- Implementation files: `src/scan/project.rs`, `benches/scan_throughput.rs` only.
- Keep lazy initialization after existing candidate filters; do not add eager work.
- Do not change risk formula, lockfile list/order, explain, classifier, safety, selection, ActionPlan,
  clean/delete, dependencies or private advisory artifacts.
- Implementation starts from merged Spec PR on latest `origin/main`.
- Merge only with fresh current-head gates under standing authorization; never force push.
