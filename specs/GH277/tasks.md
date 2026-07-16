# GH277 Tasks

## Linked Artifacts

- Issue: `#277`
- Product spec: `specs/GH277/product.md`
- Tech spec: `specs/GH277/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH277 Spec PR to merge.

## Implementation Tasks

### SP277-T1 — Capture the broken JSON baseline

- Owner: `implementation`
- Dependencies: merged GH277 Spec PR; latest `origin/main`
- Covers: B-001, B-004, B-005, B-006, B-009
- Change: add E2E cases for met, shortfall and positive/zero-target no-candidate JSON results before
  production changes.
- Done when: current main fails JSON parsing or schema assertions while unchanged human tests pass.
- Verify: exact failing focused tests with stdout evidence.

### SP277-T2 — Add the borrowed free proposal schema

- Owner: `implementation`
- Dependencies: SP277-T1
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-010
- Change: derive the private camelCase output view from target, proposal, plan path and existing
  `Candidate` references; serialize it as one document.
- Done when: six top-level fields are exact, candidates use their existing schema, null path is
  explicit, zero-target empty proposals remain false, and serialization happens before stdout
  emission.
- Verify: focused E2E key/type/value/order assertions plus source/diff review.

### SP277-T3 — Separate human and JSON control flow

- Owner: `implementation`
- Dependencies: SP277-T1, SP277-T2
- Covers: B-001, B-004, B-005, B-006, B-007, B-008, B-009, B-010
- Change: preserve interactive gates and human strings; emit JSON for each non-interactive outcome;
  write plans before JSON for non-empty proposals.
- Done when: JSON stdout stays pure across 0/3 exits, plan errors leave stdout empty, and all prior
  human/interactive behavior remains green.
- Verify: focused free_output suite and exact before/after behavior assertions.

### SP277-T4 — Document the machine contract

- Owner: `implementation`
- Dependencies: SP277-T2, SP277-T3
- Covers: B-002, B-003, B-004, B-005, B-006, B-010
- Change: add a README example and architecture schema note without redefining Candidate fields.
- Done when: docs identify version, result fields, exit-code semantics and ActionPlan relationship.
- Verify: docs/source contract comparison, links and `cargo fmt -- --check`.

## Verification And Handoff Tasks

### SP277-T5 — Full gate, VibeGuard and SpecRail audit

- Owner: `verification`
- Dependencies: SP277-T1, SP277-T2, SP277-T3, SP277-T4
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010
- Done when: focused, stable, release, exact MSRV, VibeGuard and current-head PR gates pass with no
  spec mismatch, undeclared output field or extra path.
- Verify:
  - `cargo test --test cli free_output -- --nocapture`
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

- Serialize the existing Candidate; do not introduce a parallel candidate DTO.
- Keep planPath present and null when no plan is written.
- Do not print JSON until a non-empty proposal's ActionPlan write succeeds.
- Preserve every existing human output string and exit code.
- Start implementation from the merged Spec PR on latest `origin/main`.
