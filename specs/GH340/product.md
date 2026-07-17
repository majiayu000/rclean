# GH340 Product Spec

## Linked Issue

- `#340` — `refactor(test): externalize scan walker tests`

## Problem

`src/scan/walker.rs` is 458 lines. Its production traversal, accumulator, and poisoned-lock logic ends at line 337,
while the final 120 lines contain an inline four-test module. This keeps lock-poison recovery fixtures in the
production review surface and leaves the file above the repository's 200-400 line target.

The tests cover both size and draft mutex poisoning through `WalkScratch::into_inner` and `WalkLocal` drop. The
work must therefore be a mechanical layout refactor with no traversal, error, or trust-model drift.

## Goals

1. Move the existing `scan::walker::tests` body into a private child test module.
2. Reduce `src/scan/walker.rs` below 400 lines while keeping its production prefix byte-identical.
3. Preserve all four tests, imports, fixtures, panic/error strings, candidate draft fields, and assertions.
4. Prove the move with fixed Rustfmt-normalized forward and whole-file rollback reconstruction.

## Non-Goals

- Changing traversal, concurrency, candidate classification, ignore handling, skip behavior, or symlink handling.
- Changing path safety, accumulator semantics, poisoned-lock detection/error propagation, sizes, drafts, or output.
- Adding, removing, renaming, rewriting, skipping, or weakening tests, fixtures, panic messages, or assertions.
- Changing public APIs, visibility, dependencies, features, workflows, docs, rules, or trust-model policy.
- Adding aliases, wrappers, lint suppressions, new helpers, or abstractions.

## Acceptance Criteria

### B-001 — Production source is unchanged

Baseline `src/scan/walker.rs:1-339` remains byte-identical. The parent changes only by replacing the inline
`mod tests { ... }` wrapper with `mod tests;` after the existing `#[cfg(test)]` attribute.

### B-002 — Test body is mechanically preserved

The 117-line baseline body moves to `src/scan/walker/tests.rs` after removing one four-space module nesting level
and applying crate-edition Rust 2024 formatting. No other content normalization is allowed.

### B-003 — Test contracts are preserved

The `scan::walker::tests` namespace, four test names, imports, scratch/local fixtures, poison panics, candidate
draft data, error strings, catch-unwind boundaries, and assertions remain semantically and textually equivalent
under the fixed Rustfmt proof.

### B-004 — Scope and size are bounded

Implementation changes exactly `src/scan/walker.rs` and adds `src/scan/walker/tests.rs`. The parent is 340 lines,
the child is 117 lines, and no broader visibility or production module surface is added.

### B-005 — Behavior and adjacent surfaces do not drift

Production scan/walker/symlink/poison behavior, other source/tests, public APIs, dependencies, features, workflows,
docs, rules, and trust-model policy remain unchanged.

### B-006 — Merge evidence is fresh

Exact forward/rollback proofs, focused stable and exact Rust 1.95.0 tests, full local gates, VibeGuard guards,
independent review, current-head four-platform CI, valid signatures, reviewThreads, and the SpecRail required PR
gate all pass before merge.

## Done When

All B-001 through B-006 criteria are satisfied, the implementation PR merges from its verified exact head, and
Issue #340 closes.
