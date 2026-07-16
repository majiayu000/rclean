# GH265 Tasks

## Linked Artifacts

- Issue: `#265`
- Product spec: `specs/GH265/product.md`
- Tech spec: `specs/GH265/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH265 Spec PR to merge.

## Implementation Tasks

### SP265-T1 — Capture source and test baselines

- Owner: `implementation`
- Dependencies: merged GH265 Spec PR; latest `origin/main`
- Covers: B-001, B-005
- Change: record the production/test boundary, parent line count, three test names, and unchanged
  bodies of the two anchor-state tests.
- Done when: evidence can detect production or preserved-test drift.
- Verify: source inspection and `cargo test doctor::tests -- --list`.

### SP265-T2 — Externalize the doctor child test module

- Owner: `implementation`
- Dependencies: SP265-T1
- Covers: B-001, B-005, B-006
- Change: replace inline tests with `#[cfg(test)] mod tests;` and move the body to
  `src/doctor/tests.rs` without production visibility changes.
- Done when: parent <700 lines, child <800 lines, and only two manifest paths differ.
- Verify: prefix/body comparison, `wc -l`, `git diff --check`, scope diff.

### SP265-T3 — Replace count-only coverage with exact identity

- Owner: `implementation`
- Dependencies: SP265-T2
- Covers: B-002, B-003, B-004, B-005
- Change: replace only the first test with catalog-derived `BTreeSet` equality and a separate
  duplicate-ID assertion; remove the fixed 59 assertion.
- Done when: three focused tests pass and failures would expose duplicate/missing/extra IDs.
- Verify: `cargo test doctor::tests` and implementation diff review.

## Verification And Handoff Tasks

### SP265-T4 — Full gate, VibeGuard and SpecRail audit

- Owner: `verification`
- Dependencies: SP265-T1, SP265-T2, SP265-T3
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007
- Done when: stable/release/MSRV/VibeGuard/current-head CI pass and spec-vs-implementation has no
  missing, mismatch or extra scope.
- Verify:
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

- Do not modify doctor production entries, rule catalog, classifiers or visibility.
- Preserve HOME guard and the two anchor-state test bodies.
- Change only the first test and the module location.
- Start implementation from the merged Spec PR on latest `origin/main`.
