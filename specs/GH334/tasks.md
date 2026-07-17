# GH334 Tasks

## Linked Artifacts

- Issue: `#334`
- Product: `specs/GH334/product.md`
- Tech: `specs/GH334/tech.md`
- Route after approval: `implement`

## Status

`planned` — implementation starts only after this Spec PR merges.

## SpecRail Checklist

- [ ] `SP334-T1` | Owner: `free` | Done when: the parent retains its exact production prefix and declares the private external test module | Verify: production-prefix hash/diff + exact declaration + line count
- [ ] `SP334-T2` | Owner: `free-tests` | Done when: the 45-line test body moves with only the fixed four-space dedent and exact rollback reconstruction | Verify: raw/normalized hashes + child diff + rollback diff + focused stable/MSRV tests
- [ ] `SP334-T3` | Owner: `verification` | Done when: scope, unchanged support/APIs, full local and remote gates pass | Verify: scope/full/VibeGuard/CI/review/PR gates

## SP334-T1 — Externalize the module boundary

- Dependencies: merged Spec; latest main; unchanged baseline hashes in `tech.md`
- Covers: B-001, B-004, B-005
- Change:
  - keep `src/free.rs:1-379` byte-identical;
  - replace only the inline wrapper with `#[cfg(test)] mod tests;`;
  - add no import, visibility, helper, cfg, comment, or production change.
- Done when: the parent prefix hash/diff is exact, the declaration occurs once, and the parent is 381 lines.
- Verify: fixed prefix proof, declaration search, public/scope audit, line count.

## SP334-T2 — Move the exact test body

- Dependencies: SP334-T1
- Covers: B-002, B-003, B-004, B-005
- Change:
  - add `src/free/tests.rs` from baseline lines 382-426;
  - remove exactly one four-space nesting indent from the 42 nonblank lines;
  - preserve blank lines, order, imports, attributes, names, fixture calls, inputs, comments, and assertions.
- Done when: raw/normalized hashes match, the child diff and rollback reconstruction are empty, test support remains
  unchanged, and all three tests run once under stable and exact Rust 1.95.0.
- Verify: fixed forward/rollback proof, test/support hashes, name inventory, focused stable/MSRV tests.

## SP334-T3 — Prove merge readiness

- Dependencies: SP334-T1 and SP334-T2
- Covers: B-001, B-002, B-003, B-004, B-005, B-006
- Done when: exact scope/source evidence, all stable and exact MSRV gates, VibeGuard, independent review, and final
  current-head remote evidence pass.
- Verify:
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - all eight installed Rust VibeGuard guards plus test-integrity, test-weakening, and dependency-change guards
  - signed head, independent review, current-head four-check CI, reviewThreads, and SpecRail required PR gate

## Invariant Coverage Audit

- Product: `{B-001, B-002, B-003, B-004, B-005, B-006}`
- Tasks: `{B-001, B-002, B-003, B-004, B-005, B-006}`
- Missing: `none`

## Handoff Notes

- Start implementation only from the latest `origin/main` after the Spec merges.
- Change exactly `src/free.rs` and add `src/free/tests.rs`; do not touch production logic or test support.
- Preserve the 379-line prefix byte-for-byte and move the 45-line body with only the fixed dedent.
- Keep the `free::tests` namespace, three names, fixtures, inputs, comments, and assertions exact.
- Fresh local and remote gates plus standing merge authorization are required; never force push.
