# GH314 Tasks

## Linked Artifacts

- Issue: `#314`
- Product spec: `specs/GH314/product.md`
- Tech spec: `specs/GH314/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH314 Spec PR to merge.

## SpecRail Checklist

- [ ] `SP314-T1` | Owner: `refactor` | Done when: inline project-rule tests move into the standard child module with exact 384/245 line contracts | Verify: prefix/normalized-body diff and focused tests
- [ ] `SP314-T2` | Owner: `verification` | Done when: zero semantic drift and full local/remote gates pass | Verify: stable/MSRV/VibeGuard/CI/PR gates

## Implementation Tasks

### SP314-T1 — Extract the inline test module mechanically

- Owner: `refactor`
- Dependencies: merged GH314 Spec PR; unchanged `origin/main` project-rule baseline
- Covers: B-001, B-002, B-003, B-004, B-005
- Change: preserve parent lines 1–383, replace the inline wrapper with `mod tests;`, create
  `src/rules/project/tests.rs` from baseline lines 385–629 with one four-space dedent, then apply repository
  rustfmt normalization.
- Done when: parent is 384 lines, child is 245 lines, both exact proofs are empty, and only the two planned paths
  appear in the diff.
- Verify:
  - exact relocation proof from `specs/GH314/tech.md`
  - source scan for position-sensitive constructs
  - `git diff --check`
  - `git diff --name-status origin/main...HEAD`
  - `cargo fmt -- --check`
  - `cargo test rules::project::tests -- --nocapture`

## Verification And Handoff Tasks

### SP314-T2 — Prove zero semantic drift and merge readiness

- Owner: `verification`
- Dependencies: SP314-T1
- Covers: B-001, B-002, B-003, B-004, B-005, B-006
- Done when: moved content equals the one-level-dedented and same-rustfmt-normalized baseline, nine focused tests
  and full stable/MSRV gates pass, no production/dependency/workflow drift exists, and final-head
  review/CI/SpecRail gates are green.
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

- Implementation files: `src/rules/project.rs` and `src/rules/project/tests.rs` only.
- Do not mix test cleanup, naming, deduplication or classifier refactoring into the move.
- If `origin/main` changes the baseline layout, stop and refresh proof coordinates.
- Merge only with fresh current-head gates under standing authorization; never force push.
