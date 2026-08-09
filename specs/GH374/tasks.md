# GH374 Tasks

## Linked Artifacts

- Issue: `#374`
- Product spec: `specs/GH374/product.md`
- Tech spec: `specs/GH374/tech.md`
- Route: `plan_first`

## Status

`approved_for_implementation` — verified output-determinism bug with no
trust-model behavior change.

## Implementation Tasks

### SP374-T1 — Record the failing ordering case

- [x] Insert distinct warnings in non-canonical order.
- [x] Assert exact canonical order and unchanged count.
- Covers: B-001, B-002, B-003.

### SP374-T2 — Canonicalize at the merge boundary

- [x] Reuse the sizing comparator from the walker merge.
- [x] Keep all traversal and warning generation unchanged.
- Covers: B-001 through B-005.

### SP374-T3 — Document and verify

- [x] Update the changelog and packet index.
- [x] Run focused, full, release, and Rust 1.95 gates.
- Covers: B-001 through B-005.

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005}`
- Missing invariants: `none`
