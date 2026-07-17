# GH329 Tasks

## Linked Artifacts

- Issue: `#329`
- Product: `specs/GH329/product.md`
- Tech: `specs/GH329/tech.md`
- Route after approval: `implement`

## Status

`planned` — implementation resumes only after the GH329 Clippy-proof Spec correction merges.

## SpecRail Checklist

- [ ] `SP329-T1` | Owner: `doctor` | Done when: the common ordered entry prefix moves into one private child with only the fixed 12-token borrow normalization | Verify: normalized common extraction proof + focused stable/MSRV tests
- [ ] `SP329-T2` | Owner: `doctor` | Done when: the platform suffix moves into one private child with only the hash-pinned Windows parameter-consumption prelude and the parent preserves HOME/Docker orchestration | Verify: prelude/platform/parent source proofs + focused stable/MSRV tests
- [ ] `SP329-T3` | Owner: `verification` | Done when: scope, sizes, unchanged tests/APIs, full local and remote gates pass | Verify: scope/full/VibeGuard/CI/review/PR gates

## SP329-T1 — Extract common entries

- Dependencies: merged Spec; latest main; unchanged baseline hashes in `tech.md`
- Covers: B-001, B-002, B-005, B-006
- Change:
  - add `doctor/common_entries.rs` with exactly one `pub(super)` collector;
  - move the existing common sequence without changing values, order, cfg predicates, or effective helper
    arguments, replacing exactly 12 now-redundant `&home` expressions with `home`;
  - return the constructed vector without adding a registry or wrapper.
- Done when: the fixed 12-token normalization has the recorded hash, inverse extraction reproduces that normalized
  baseline exactly, and the child remains below 400 lines.
- Verify: counted/hash-pinned normalized common source proof, visibility search, line count, focused stable/MSRV
  doctor tests.

## SP329-T2 — Extract platform entries and retain orchestration

- Dependencies: SP329-T1
- Covers: B-001, B-003, B-004, B-005, B-006, B-007
- Change:
  - add `doctor/platform_entries.rs` with exactly one `pub(super)` appender;
  - move the existing platform sequence without changing values, order, cfg predicates, or fallbacks;
  - prepend only the specified Windows `let _ = home;` cfg block so all-platform Clippy passes without a lint
    suppression or signature change;
  - reduce parent orchestration to HOME resolution, common collection, platform append, optional-last Docker append,
    and report return;
  - retain all types, APIs, low-level helpers, and the external test module in the parent.
- Done when: the platform prelude hash matches, the platform and helper-tail reconstruction diffs are empty, tests
  remain byte-identical, and all three affected production files are below 400 lines.
- Verify: exact prelude/platform/parent/test proofs, path/API/visibility audits, focused stable/MSRV doctor tests.

## SP329-T3 — Prove merge readiness

- Dependencies: SP329-T1 and SP329-T2
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008
- Done when: exact scope/source evidence, all stable and exact MSRV gates, VibeGuard, independent review, and final
  current-head remote evidence pass.
- Verify:
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - all eight installed Rust VibeGuard guards and exact change-integrity proofs
  - signed head, independent review, current-head four-check CI, reviewThreads and SpecRail required PR gate

## Invariant Coverage Audit

- Product: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008}`
- Tasks: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008}`
- Missing: `none`

## Handoff Notes

- Start implementation only from the latest `origin/main` after this Spec merges.
- Keep exact three-path scope; do not edit tests, anchors, rules, dependencies, workflows, or docs.
- Preserve common/platform body order and cfg predicates mechanically; only the 12 approved common borrow tokens
  and the hash-pinned Windows platform prelude may differ; no lint allow, registry, macro, alias, trait, or builder.
- Keep exactly two new `pub(super)` entry points and no new broader visibility.
- Fresh local and remote gates plus standing merge authorization are required; never force push.
