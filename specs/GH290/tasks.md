# GH290 Tasks

## Linked Artifacts

- Issue: `#290`
- Product spec: `specs/GH290/product.md`
- Tech spec: `specs/GH290/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH290 Spec PR to merge.

## Implementation Tasks

### SP290-T1 — Lock audit enum serialization contract

- Owner: `implementation`
- Dependencies: merged GH290 Spec PR; latest `origin/main`
- Covers: B-002, B-003, B-004
- Change: add focused serde JSON assertions for all base variants and feature-enabled
  `Graveyard`/`Skipped` variants without changing production behavior.
- Done when: exact snake_case enum strings are proven in default and feature-specific builds.
- Verify: `cargo test clean::audit::tests`

### SP290-T2 — Align graveyard-only variants with feature boundary

- Owner: `implementation`
- Dependencies: SP290-T1
- Covers: B-001, B-002, B-004, B-005, B-006
- Change: add `#[cfg(feature = "graveyard")]` to the two exact variants; do not modify call sites,
  deletion flow, schemas or lint configuration.
- Done when: warnings-as-errors no-default build passes and enabled serialization tests remain green.
- Verify:
  - `RUSTFLAGS='-D warnings' cargo test --no-default-features --no-run`
  - `git diff --name-only origin/main...HEAD`

## Verification And Handoff Tasks

### SP290-T3 — Prove the full feature matrix is warning-clean

- Owner: `verification`
- Dependencies: SP290-T2
- Covers: B-001, B-002, B-003, B-004, B-007
- Done when: no-default, tui-only, graveyard-only and all-feature warnings-as-errors builds pass.
- Verify:
  - `RUSTFLAGS='-D warnings' cargo test --no-default-features --no-run`
  - `RUSTFLAGS='-D warnings' cargo test --no-default-features --features tui --no-run`
  - `RUSTFLAGS='-D warnings' cargo test --no-default-features --features graveyard --no-run`
  - `RUSTFLAGS='-D warnings' cargo test --all-features --no-run`

### SP290-T4 — Full stable/MSRV/VibeGuard/SpecRail gate

- Owner: `verification`
- Dependencies: SP290-T3
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007
- Done when: scope is exactly `src/clean/audit.rs`; stable/release/MSRV/VibeGuard/current-head PR
  gates pass; spec-vs-implementation finds no deletion behavior or private-advisory drift.
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

- One-file implementation scope: `src/clean/audit.rs` only.
- Do not modify deletion call sites, audit schema, CLI, dependencies, workflows or safety gates.
- Do not suppress dead-code warnings; align declarations with the existing feature boundary.
- Start implementation only after the Spec PR merges on latest `origin/main`.
- Merge only with fresh current-head gates under standing authorization; never force push.
