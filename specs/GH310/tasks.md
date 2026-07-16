# GH310 Tasks

## Linked Artifacts

- Issue: `#310`
- Product spec: `specs/GH310/product.md`
- Tech spec: `specs/GH310/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH310 Spec PR to merge.

## SpecRail Checklist

- [ ] `SP310-T1` | Owner: `refactor` | Done when: inline deletion test body is moved exactly into the standard child module and both files meet exact line contracts | Verify: prefix/body diff and line-count proof
- [ ] `SP310-T2` | Owner: `verification` | Done when: zero-semantic-drift, full stable/MSRV, VibeGuard, CI and PR gates pass | Verify: focused/full commands and current-head remote evidence

## Implementation Tasks

### SP310-T1 — Extract the inline test module mechanically

- Owner: `refactor`
- Dependencies: merged GH310 Spec PR; unchanged `origin/main` deletion baseline
- Covers: B-001, B-002, B-003, B-004, B-005
- Change: preserve parent lines 1–307, replace the inline wrapper with `mod tests;`, create
  `src/clean/deletion/tests.rs` from baseline lines 309–545 with exactly one four-space dedent, then apply the
  repository rustfmt normalization.
- Done when: parent is 308 lines, rustfmt-normalized child is 229 lines, both exact diff proofs are empty, and
  only the two planned paths appear in the diff.
- Verify:
  - exact relocation proof from `specs/GH310/tech.md`
  - `git diff --check`
  - `git diff --name-status origin/main...HEAD`
  - `cargo fmt -- --check`
  - `cargo test clean::deletion::tests -- --nocapture`

## Verification And Handoff Tasks

### SP310-T2 — Prove zero semantic drift and merge readiness

- Owner: `verification`
- Dependencies: SP310-T1
- Covers: B-001, B-002, B-003, B-004, B-005, B-006
- Done when: all moved content is textually identical after dedent, focused/full stable/MSRV tests pass, no
  production/dependency/workflow drift exists, and final-head review/CI/SpecRail gates are green.
- Verify:
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - all eight Rust VibeGuard guards
  - VibeGuard test-integrity, test-weakening and dependency guards
  - SpecRail packet check, required PR gate, signatures, current-head CI and reviewThreads

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006}`
- Missing invariants: `none`

## Handoff Notes

- Implementation files: `src/clean/deletion.rs` and `src/clean/deletion/tests.rs` only.
- Do not mix test cleanup, renaming, deduplication or production refactoring into the move.
- If `origin/main` changes the baseline line layout, stop and refresh the spec proof rather than forcing the old
  extraction coordinates.
- Merge only with fresh current-head gates under standing authorization; never force push.
