# GH326 Tasks

## Linked Artifacts

- Issue: `#326`
- Product: `specs/GH326/product.md`
- Tech: `specs/GH326/tech.md`
- Route after approval: `implement`

## Status

`planned` — implementation waits for merged GH326 Spec PR.

## SpecRail Checklist

- [ ] `SP326-T1` | Owner: `tests` | Done when: one cfg(test)-only fixture module replaces both local constructor pairs without semantic drift | Verify: exact/normalized source proof + focused stable/MSRV tests
- [ ] `SP326-T2` | Owner: `verification` | Done when: bounded scope, production exclusion, full gates, reviewer and remote evidence pass | Verify: source/full/VibeGuard/CI/PR gates

## SP326-T1 — Centralize the ranking fixtures

- Dependencies: merged Spec; latest main; unchanged baseline constructors and six-test inventory
- Covers: B-001, B-002, B-003, B-004, B-005, B-006
- Change:
  - add the gated `test_support` module and its two crate-private typed constructors;
  - remove the duplicate local helpers from `free` and `output` tests;
  - use direct shared function names, retaining explicit free safety cases and adding explicit `Safety::Safe` to output.
- Done when: the exact four-file diff satisfies the source and semantic proofs with no assertion, production,
  dependency, workflow, or test-name drift.
- Verify:
  - exact path/helper/module/count commands from `tech.md`
  - base-prefix and normalized/inverse fixture proofs
  - focused stable and Rust 1.95.0 tests

## SP326-T2 — Prove production exclusion and merge readiness

- Dependencies: SP326-T1
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007
- Done when: release/full stable and exact MSRV builds/tests pass and final remote evidence is current.
- Verify:
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - all eight Rust VibeGuard guards and change-integrity guards
  - exact signature, independent review, current-head four-check CI, reviewThreads and SpecRail required PR gate

## Invariant Coverage Audit

- Product: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007}`
- Tasks: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007}`
- Missing: `none`

## Handoff Notes

- Start implementation only from the latest `origin/main` after this Spec merges.
- Keep the exact four-path scope and do not edit production prefixes or assertions.
- Use direct function names; no aliases, wrappers, builders, macros, or dependencies.
- Preserve the six-test inventory and explicit safety semantics.
- Fresh local and remote gates plus standing merge authorization are required; never force push.
