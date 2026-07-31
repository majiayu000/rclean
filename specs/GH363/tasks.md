# GH363 Tasks

## Linked Artifacts

- Pull request: `#363`
- Product spec: `specs/GH363/product.md`
- Tech spec: `specs/GH363/tech.md`
- Route: `implement`
- Commit policy: `per_step`

## Status

`remote_gate_in_progress` — maintainer authorization was received in the active
goal thread. RED/GREEN and the complete local repository/MSRV gate passed on
the branch integrated with the latest `main`. The original branch is pushed
and its blocking review thread is resolved. GitHub remains authoritative for
the current-head CI and merge/closure state.

## Implementation Tasks

### SP363-T1 — SpecRail packet

- [x] Add product, technical, and task contracts under `specs/GH363/`.
- Covers: B-001 through B-006.
- Done when: planned and forbidden paths, RED/GREEN test, and full verification
  commands are explicit.
- Verify:
  - `git diff --check`
  - `cargo fmt -- --check`

### SP363-T2 — Failing replay regression

- [x] Add the Unix unreadable-descendant plan replay test.
- Covers: B-002, B-003, B-004.
- Done when: the focused test fails against the current production code because
  replay incorrectly succeeds with partial bytes.
- Verify:
  - `cargo test plan::tests::revalidation_rejects_incomplete_current_size -- --exact`

### SP363-T3 — Strict replay sizing

- [x] Make the replay-only sizer adapter return all warnings as an error.
- [x] Map those warnings to contextual `PlanError` in `revalidate_selected`.
- Covers: B-001 through B-005.
- Done when: the RED test becomes GREEN and the existing stale-byte test stays
  green without changing ordinary scan behavior.
- Verify:
  - `cargo test plan::tests::revalidation_rejects_incomplete_current_size -- --exact`
  - `cargo test plan::tests::revalidation_updates_stale_bytes_from_disk -- --exact`
  - `cargo test scan::sizer::tests`

### SP363-T4 — Scope and complete repository gate

- [x] Confirm no forbidden file is modified.
- [x] Run formatting, clippy, full tests, release build, and Rust 1.95 gates.
- Covers: B-001 through B-006.
- Verify:
  - `git diff --check`
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95 cargo build --all-targets --all-features`
  - `rustup run 1.95 cargo test`

## Local Verification Evidence

- SpecRail commit: `121616c`
- Production and regression commit: `a5e7c86`
- RED: the new replay regression returned success with only 7 readable bytes.
- GREEN: the same regression rejected two unreadable descendants and preserved
  both paths in the error.
- Full gate: fmt, clippy, all tests, release build, Rust 1.95 all-targets build,
  and Rust 1.95 tests passed.

## Remote Tasks

### SP363-T5 — Original PR branch update

Completed before this spec snapshot:

- signed commits were created on `fix/audit-h5-h8-c1`;
- remote head was refreshed and matched before each fast-forward push;
- the branch was integrated with the latest `main` without force-push;
- the existing review thread was answered with RED/GREEN/full-gate evidence
  and resolved;
- an independent current-head reviewer reported no blocking findings.

The remaining current-head CI state is intentionally recorded in GitHub rather
than pre-marked in a commit that would itself trigger a newer CI run.

### SP363-T6 — Merge and closure

Operational closure gate:

- re-check head SHA, CI, merge state, and GraphQL review threads;
- merge only after every gate is clean;
- confirm remote PR closure and branch state.

These are live GitHub transitions performed after the final commit. Their
authoritative evidence belongs in PR #363 and the queue closure ledger, not in
checkboxes that would be stale as soon as this file changed.

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006}`
- Missing invariants: `none`
