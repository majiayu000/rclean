# GH337 Product Spec

## Linked Issue

- `#337` — `refactor(test): externalize scan project tests`

## Problem

`src/scan/project.rs` is 436 lines. Its production activity, report, and risk logic ends at line 306, while the
final 129 lines contain an inline five-test module. This keeps performance-contract fixtures and cross-platform
traversal tests in the production review surface and leaves the file above the repository's 200-400 line target.

The tests cover lazy risk caching, activity ordering, missing paths, traversal boundaries, and cfg-specific
symlink fixtures. The work must therefore be a mechanical layout refactor with no scanning or trust-model drift.

## Goals

1. Move the existing `scan::project::tests` body into a private child test module.
2. Reduce `src/scan/project.rs` below 400 lines while keeping its production prefix byte-identical.
3. Preserve all five tests, their helper/imports/fixtures/cfg branches/inputs/comments/assertions, and namespace.
4. Prove the move with fixed rustfmt-normalized forward and whole-file rollback reconstruction.

## Non-Goals

- Changing project activity traversal, depth/skip boundaries, concurrency, missing-path behavior, or ordering.
- Changing symlink handling, risk-score calculation/cache behavior, candidate/report construction, or warnings.
- Adding, removing, renaming, rewriting, skipping, or weakening tests, helpers, fixtures, or assertions.
- Changing public APIs, visibility, dependencies, features, workflows, docs, rules, or trust-model policy.
- Adding aliases, wrappers, lint suppressions, new helpers, or abstractions.

## Acceptance Criteria

### B-001 — Production source is unchanged

Baseline `src/scan/project.rs:1-308` remains byte-identical. The parent changes only by replacing the inline
`mod tests { ... }` wrapper with `mod tests;` after the existing `#[cfg(test)]` attribute.

### B-002 — Test body is mechanically preserved

The 126-line baseline body moves to `src/scan/project/tests.rs` after removing one four-space module nesting level
and applying crate-edition Rust 2024 formatting. No other content normalization is allowed.

### B-003 — Test contracts are preserved

The `scan::project::tests` namespace, five test names, `write_with_modified` helper, imports, fixture paths,
activity/risk inputs, cfg branches, symlink calls, comments, and assertions remain semantically and textually
equivalent under the fixed rustfmt proof.

### B-004 — Scope and size are bounded

Implementation changes exactly `src/scan/project.rs` and adds `src/scan/project/tests.rs`. The parent is 309
lines, the child is 125 lines, and no broader visibility or production module surface is added.

### B-005 — Behavior and adjacent surfaces do not drift

Production scan/risk/symlink behavior, other source/tests, public APIs, dependencies, features, workflows, docs,
rules, and trust-model policy remain unchanged.

### B-006 — Merge evidence is fresh

Exact forward/rollback proofs, focused stable and exact Rust 1.95.0 tests, full local gates, VibeGuard guards,
independent review, current-head four-platform CI, valid signatures, reviewThreads, and the SpecRail required PR
gate all pass before merge.

## Done When

All B-001 through B-006 criteria are satisfied, the implementation PR merges from its verified exact head, and
Issue #337 closes.
