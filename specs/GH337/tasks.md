# GH337 Tasks

## Linked Artifacts

- Issue: `#337`
- Product: `specs/GH337/product.md`
- Tech: `specs/GH337/tech.md`
- Route after approval: `implement`

## Status

`planned` — implementation starts only after this Spec PR merges.

## SpecRail Checklist

- [ ] `SP337-T1` | Owner: `scan-project` | Done when: the exact 308-line parent prefix remains and the private child module is declared | Verify: prefix hash/diff + exact declaration + 309-line count
- [ ] `SP337-T2` | Owner: `scan-project-tests` | Done when: the five-test body moves through only the fixed dedent/Rust 2024 normalization and whole-file rollback is exact | Verify: raw/dedented/formatted hashes + child/rollback diffs + focused stable/MSRV tests
- [ ] `SP337-T3` | Owner: `verification` | Done when: scope, production/trust invariants, and all local/remote gates pass | Verify: scope/full/VibeGuard/CI/review/PR gates

## SP337-T1 — Externalize the module boundary

- Dependencies: merged Spec; latest main; unchanged baseline hashes in `tech.md`
- Covers: B-001, B-004, B-005
- Change:
  - keep `src/scan/project.rs:1-308` byte-identical;
  - replace only the inline wrapper/body with `mod tests;`;
  - add no production import, visibility, helper, cfg, comment, or whitespace change.
- Done when: prefix hash/diff is exact, line 309 is `mod tests;`, and the parent is 309 lines.
- Verify: fixed prefix proof, declaration search, public/scope audit, line count.

## SP337-T2 — Move the rustfmt-normalized test body

- Dependencies: SP337-T1
- Covers: B-002, B-003, B-004, B-005
- Change:
  - add `src/scan/project/tests.rs` from baseline lines 310-435;
  - remove one four-space nesting indent, then apply Rust 2024 rustfmt;
  - preserve the five tests, helper, imports, fixtures, cfg/symlink calls, inputs, comments, and assertions.
- Done when: all three source hashes match, child diff and whole-file rollback are empty, no adjacent file changes,
  and stable/exact Rust 1.95.0 focused runs pass 5/5.
- Verify: fixed forward/rollback proof, inventory/visibility search, focused stable/MSRV tests.

## SP337-T3 — Prove merge readiness

- Dependencies: SP337-T1 and SP337-T2
- Covers: B-001, B-002, B-003, B-004, B-005, B-006
- Done when: exact scope/source evidence, stable and exact MSRV full gates, VibeGuard, independent review, and final
  current-head remote evidence pass.
- Verify:
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - all eight Rust VibeGuard guards plus test-integrity, test-weakening, and dependency-change guards
  - signed head, independent review, current-head four-check CI, reviewThreads, and SpecRail required PR gate

## Invariant Coverage Audit

- Product: `{B-001, B-002, B-003, B-004, B-005, B-006}`
- Tasks: `{B-001, B-002, B-003, B-004, B-005, B-006}`
- Missing: `none`

## Handoff Notes

- Start implementation only from the latest `origin/main` after the Spec merges.
- Change exactly `src/scan/project.rs` and add `src/scan/project/tests.rs`; do not touch production or other tests.
- Preserve the 308-line prefix byte-for-byte; use only the pinned dedent plus Rust 2024 formatting for the body.
- Preserve all five tests and the cross-platform symlink fixture contract; no trust-model behavior changes.
- Fresh local and remote gates plus standing merge authorization are required; never force push.
