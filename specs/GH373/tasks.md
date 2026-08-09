# GH373 Tasks

## Linked Artifacts

- Issue: `#373`
- Product spec: `specs/GH373/product.md`
- Tech spec: `specs/GH373/tech.md`
- Route: `plan_first`

## Status

`approved_for_implementation` — the issue has a verified root cause and the
design preserves all cleanup and trust-model behavior.

## Implementation Tasks

### SP373-T1 — Lock the CLI interval contract

- [x] Add typed parse assertions for the default, `5m`, and `1h`.
- [x] Add a CLI regression that rejects an age-only watch unit before scanning.
- Covers: B-001, B-002, B-003, B-004, B-005.

### SP373-T2 — Parse watch intervals at the CLI boundary

- [x] Change `WatchArgs.every` from `String` to `Duration` with the existing
  short-duration value parser.
- [x] Remove runtime age parsing from watch.
- Covers: B-001 through B-006.

### SP373-T3 — Documentation and repository gate

- [x] Record the fix in `CHANGELOG.md` and index the packet.
- [x] Run focused, full, release, and Rust 1.95 verification.
- Covers: B-001 through B-006.

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006}`
- Missing invariants: `none`
