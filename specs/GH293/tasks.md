# GH293 Tasks

## Linked Artifacts

- Issue: `#293`
- Product spec: `specs/GH293/product.md`
- Tech spec: `specs/GH293/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH293 Spec PR to merge.

## Implementation Tasks

### SP293-T1 — Add deterministic collision contract coverage

- Owner: `implementation`
- Dependencies: merged GH293 Spec PR; latest `origin/main`
- Covers: B-001, B-002, B-003, B-004, B-005
- Change: introduce the minimal fixed-stamp/probe test seam and focused tests for the first path,
  `-2`/`-3`, sentinel preservation, extensionless naming, non-UTF-8 fallback, probe error and counter
  exhaustion.
- Done when: the same-stamp regression is observed failing against old single-path behavior without
  sleep or timing assumptions.
- Verify: `cargo test watch::tests`

### SP293-T2 — Select an unused path before every watch plan write

- Owner: `implementation`
- Dependencies: SP293-T1
- Covers: B-001, B-002, B-003, B-004, B-005, B-006
- Change: format one UTC timestamp, probe the unsuffixed and numeric-suffix candidates with
  `try_exists`, use checked sequencing, then call the unchanged ActionPlan writer.
- Done when: all focused tests pass, every success message corresponds to a distinct path, and the
  diff remains inside `src/watch/mod.rs`.
- Verify:
  - `cargo test watch::tests`
  - `git diff --name-only origin/main...HEAD`

## Verification And Handoff Tasks

### SP293-T3 — Prove watch and ActionPlan compatibility

- Owner: `verification`
- Dependencies: SP293-T2
- Covers: B-001, B-002, B-003, B-004, B-005, B-006
- Done when: focused tests, existing watch state tests and repository ActionPlan/CLI suites pass;
  source inspection confirms the shared writer/schema/replay and watcher mapping are unchanged.
- Verify:
  - `cargo test watch::tests`
  - `cargo test`
  - scoped diff inspection

### SP293-T4 — Full stable/MSRV/VibeGuard/SpecRail gate

- Owner: `verification`
- Dependencies: SP293-T3
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007
- Done when: scope is exactly `src/watch/mod.rs`; stable/release/MSRV/VibeGuard/current-head PR gates
  pass; spec-vs-implementation finds no ActionPlan, deletion, watcher-selection or private-advisory
  drift.
- Verify:
  - `git diff --check`
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - all installed VibeGuard Rust guards

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007}`
- Missing invariants: `none`

## Handoff Notes

- One-file implementation scope: `src/watch/mod.rs` only.
- Do not modify plan I/O/schema/replay, scan, watcher event mapping, CLI, dependencies or safety
  gates.
- Do not claim cross-process locking; solve the reproduced serial refresh overwrite and existing-file
  collision.
- Start implementation only after the Spec PR merges on latest `origin/main`.
- Merge only with fresh current-head gates under standing authorization; never force push.
