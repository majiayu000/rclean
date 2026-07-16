# GH262 Tasks

## Linked Artifacts

- Issue: `#262`
- Product spec: `specs/GH262/product.md`
- Tech spec: `specs/GH262/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH262 Spec PR to merge.

## Implementation Tasks

### SP262-T1 — Capture immutable pre-move evidence

- Owner: `implementation`
- Dependencies: merged GH262 Spec PR; latest `origin/main`
- Covers: B-001, B-002, B-004
- Change: capture the production/test split boundary and sorted 12-test name set before editing.
- Done when: baseline evidence can detect a dropped/renamed test or production token change.
- Verify: `cargo test scan::git_cache::tests -- --list` and exact source boundary inspection.

### SP262-T2 — Move the inline test body to the child module

- Owner: `implementation`
- Dependencies: SP262-T1
- Covers: B-001, B-002, B-003, B-004, B-005, B-006
- Change: replace the inline block with `#[cfg(test)] mod tests;` and add
  `src/scan/git_cache/tests.rs` containing the exact former body.
- Done when: production content is unchanged, privacy is not widened, both files meet line limits,
  and only the two manifest paths differ.
- Verify: source/body comparison, `wc -l`, `git diff --check`, `git diff --name-only`.

### SP262-T3 — Prove test identity and behavior

- Owner: `verification`
- Dependencies: SP262-T2
- Covers: B-002, B-003, B-004, B-007
- Done when: before/after sorted name sets are identical at 12 and all focused tests pass.
- Verify:
  - `cargo test scan::git_cache::tests -- --list`
  - `cargo test scan::git_cache::tests`

## Verification And Handoff Tasks

### SP262-T4 — Full gate, VibeGuard and SpecRail audit

- Owner: `verification`
- Dependencies: SP262-T1, SP262-T2, SP262-T3
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007
- Done when: stable/release/MSRV/VibeGuard/current-head CI pass and spec-vs-implementation reports
  no missing, mismatched or extra-scope item.
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

- Do not edit production behavior or widen visibility.
- Do not rename/reformat tests beyond mechanical rustfmt output.
- Do not change any path outside the planned manifest.
- Start implementation from the merged Spec PR on latest `origin/main`.
