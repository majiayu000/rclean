# GH317 Tasks

## Linked Artifacts

- Issue: `#317`
- Product: `specs/GH317/product.md`
- Tech: `specs/GH317/tech.md`
- Route after approval: `implement`

## Status

`planned` — implementation waits for merged GH317 Spec PR.

## SpecRail Checklist

- [ ] `SP317-T1` | Owner: `refactor` | Done when: inline watch tests move to child with exact 320/229 contracts | Verify: forward proof + focused tests
- [ ] `SP317-T2` | Owner: `verification` | Done when: semantic preservation and all gates pass | Verify: stable/MSRV/VibeGuard/CI/PR gates

## SP317-T1 — Mechanical extraction

- Dependencies: merged Spec; unchanged main layout
- Covers: B-001, B-002, B-003, B-004, B-005
- Change: preserve lines 1–319, declare `mod tests;`, move lines 321–552 with one dedent and edition-2024 fmt.
- Done when: parent 320, child 229, exact diffs empty, only two planned paths changed.
- Verify: tech proof, position-macro scan, `cargo fmt -- --check`, `cargo test watch::tests -- --nocapture`.

## SP317-T2 — Verification and handoff

- Dependencies: SP317-T1
- Covers: B-001, B-002, B-003, B-004, B-005, B-006
- Done when: stable/MSRV forward+rollback proofs, focused/full gates, guards and final remote gates are green.
- Verify:
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - Rust/universal VibeGuard guards
  - SpecRail, signatures, current-head CI and reviewThreads

## Invariant Coverage Audit

- Product: `{B-001, B-002, B-003, B-004, B-005, B-006}`
- Tasks: `{B-001, B-002, B-003, B-004, B-005, B-006}`
- Missing: `none`

## Handoff Notes

- Only `src/watch/mod.rs` and `src/watch/tests.rs`.
- No cleanup, renaming, deduplication or runtime refactor.
- Refresh coordinates if main drifts.
- Fresh gates and standing authorization required; never force push.
