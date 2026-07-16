# GH274 Tasks

## Linked Artifacts

- Issue: `#274`
- Product spec: `specs/GH274/product.md`
- Tech spec: `specs/GH274/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH274 Spec PR to merge.

## Implementation Tasks

### SP274-T1 — Capture failing state-reconciliation baseline

- Owner: `implementation`
- Dependencies: merged GH274 Spec PR; latest `origin/main`
- Covers: B-001, B-002, B-003, B-005
- Change: add focused state tests for partial and empty multi-project refresh scopes before the
  production fix.
- Done when: current main behavior fails because absent in-scope projects remain, while fixtures make
  the intended retained/outside states explicit.
- Verify: exact failing `cargo test watch::tests` evidence before production edit.

### SP274-T2 — Reconcile absent projects within the refresh scope

- Owner: `implementation`
- Dependencies: SP274-T1
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008
- Change: replace the empty-report special case with one scope reconciliation pass, reuse
  `print_diff` for removals, then retain existing report insert/update behavior.
- Done when: absent in-scope keys are removed in deterministic order, present and out-of-scope keys
  are correct, and no other module changes.
- Verify: focused watch tests, source/diff inspection and exact manifest check.

### SP274-T3 — Prove path-boundary and update isolation

- Owner: `implementation`
- Dependencies: SP274-T2
- Covers: B-004, B-005, B-006, B-007
- Change: cover component-prefix collision, unrelated root preservation, empty descendant cleanup and
  current snapshot replacement with regression tests.
- Done when: `/workspace/a` cannot remove `/workspace/ab`, outside state is byte-for-byte unchanged,
  and report projects still update normally.
- Verify: `cargo test watch::tests -- --nocapture` and test review.

## Verification And Handoff Tasks

### SP274-T4 — Full gate, VibeGuard and SpecRail audit

- Owner: `verification`
- Dependencies: SP274-T1, SP274-T2, SP274-T3
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008
- Done when: focused, stable, release, exact MSRV, VibeGuard and current-head PR gates pass with no
  spec mismatch or extra implementation path.
- Verify:
  - `cargo test watch::tests -- --nocapture`
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - all installed VibeGuard Rust guards

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008}`
- Missing invariants: `none`

## Handoff Notes

- Treat every refresh root as a complete state scope, including empty reports.
- Use path-component scope checks; do not compare raw string prefixes.
- Reuse the existing removal diff path and deterministic map ordering.
- Keep implementation inside `src/watch/mod.rs` and below the 800-line ceiling.
- Start implementation from the merged Spec PR on latest `origin/main`.
