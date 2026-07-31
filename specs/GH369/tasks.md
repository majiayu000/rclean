# GH369 Tasks

## Linked Artifacts

- Issue: `#369`
- Issue URL: `https://github.com/majiayu000/rclean/issues/369`
- Implementation history: PR `#363`
- Product spec: `specs/GH369/product.md`
- Tech spec: `specs/GH369/tech.md`
- Route: `plan_first`
- Commit policy: `final_only`

## Status

`documentation_reconciled` — PR #363 shipped and verified the H5 containment
and H8 replay-sizing fixes. Issue #369 repairs the retained packet identity,
complete shipped-scope mapping, and index without changing production or test
code. GitHub remains authoritative for live issue, pull request, CI, review,
and merge state.

## Historical Implementation Tasks

### SP369-T1 — H5 fail-closed containment

- [x] Block a candidate when non-dot-root canonicalization fails.
- [x] Block a candidate that resolves outside the canonical scan root.
- [x] Preserve explicit warnings for both branches.
- Covers: B-001, B-002, B-008.
- Done when: focused safety regressions prove both blocked outcomes.
- Verify:
  - `cargo test scan::safety::tests::canonicalize_failure_blocks_candidate -- --exact`
  - `cargo test scan::safety::tests::candidate_resolving_outside_root_is_blocked -- --exact`

### SP369-T2 — Failing replay regression

- [x] Add privilege-independent adapter and plan-boundary regressions.
- Covers: B-004, B-005, B-006.
- Done when: the focused test fails against the prior production code because
  replay incorrectly succeeds with partial bytes.
- Verify:
  - `cargo test plan::tests::revalidation_rejects_incomplete_current_size -- --exact`

### SP369-T3 — Strict replay sizing

- [x] Make the replay-only sizer adapter return all warnings as an error.
- [x] Map those warnings to contextual `PlanError` in `revalidate_selected`.
- Covers: B-003 through B-007.
- Done when: the RED test becomes GREEN and the stale-byte test stays green
  without changing ordinary scan behavior.
- Verify:
  - `cargo test plan::tests::revalidation_rejects_incomplete_current_size -- --exact`
  - `cargo test plan::tests::revalidation_updates_stale_bytes_from_disk -- --exact`
  - `cargo test scan::sizer::tests`

### SP369-T4 — Historical implementation gate

- [x] Confirm no forbidden file is modified.
- [x] Run formatting, clippy, full tests, release build, and Rust 1.95 gates.
- Covers: B-001 through B-008.
- Verify:
  - `git diff --check`
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95 cargo build --all-targets --all-features`
  - `rustup run 1.95 cargo test`

## Historical Verification Evidence

- SpecRail commit: `121616c`
- Production and regression commit: `a5e7c86`
- Final PR #363 head: `e9a6a30a5f0734ea996045413b5fc1efb8deba9f`
- Squash merge: `45732fe33346b68d3b05ee3177e80d098acca2a5`
- H5 GREEN: canonicalization failure and canonical escape both produce blocked
  candidates with explicit warnings.
- H8 RED: the new replay regression returned success with only 7 readable
  bytes.
- H8 GREEN: the final deterministic regressions reject real adapter metadata
  failure and preserve two injected warning paths/messages at the plan
  boundary.
- Full gate: fmt, clippy, all tests, release build, Rust 1.95 all-targets build,
  and Rust 1.95 tests passed.

## Documentation Reconciliation Tasks

### SP369-T5 — Issue-keyed packet and complete scope

- [x] Relocate the PR-keyed packet to `specs/GH369/`.
- [x] Make issue #369 the stable identifier and retain PR #363 as
  implementation history.
- [x] Record both H5 and H8 behavior, implementation paths, focused tests, and
  historical verification.
- Covers: B-001 through B-008.

### SP369-T6 — Retained index and docs-only verification

- [x] Update `specs/README.md` to 50 directories and 150 packet files.
- [x] Add the GH369 row without duplicating the packet.
- [x] Confirm the follow-up changes no production or test file.
- Verify:
  - `git diff --check`
  - `cargo fmt -- --check`
  - exact 50-directory / 150-file count checks
  - link and stale-reference checks

## Live Remote Closure

The follow-up pull request must link and close issue #369. Current-head CI,
independent review, connector threads, merge, and issue closure are live GitHub
transitions; their authoritative state belongs in GitHub rather than
pre-marked checkboxes in this retained historical packet.

## Invariant Coverage Audit

- Product invariant set:
  `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008}`
- Task coverage union:
  `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008}`
- Missing invariants: `none`
