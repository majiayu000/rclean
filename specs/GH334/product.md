# GH334 Product Spec

## Linked Issue

- `#334` — `refactor(test): externalize free selection tests`

## Problem

After GH326 centralized ranking fixtures, `src/free.rs` is still 427 lines. Its production implementation ends at
line 379, while the final 48 lines contain an inline three-test module. This keeps test-only setup and assertions
inside a production file that remains above the repository's 200-400 line target.

The tests protect safe-only target selection and ranking behavior, so the change must be a mechanical layout
refactor. It must not alter production selection, ActionPlan, interactive cleanup, output, or any test contract.

## Goals

1. Move the existing `free::tests` body into a dedicated private child test module.
2. Reduce `src/free.rs` below 400 lines without changing its production prefix.
3. Preserve all three test names, fixtures, inputs, comments, assertions, and namespace.
4. Prove the move through fixed source reconstruction plus stable, MSRV, and cross-platform gates.

## Non-Goals

- Changing `free` ranking, safe-only filtering, pruning, target status, or output.
- Changing ActionPlan creation/replay, interactive selection, cleanup, delete mode, or recovery behavior.
- Adding, removing, renaming, or strengthening tests or assertions.
- Changing `src/test_support.rs`, public APIs, dependencies, features, workflows, docs, rules, or trust-model policy.
- Adding helpers, aliases, compatibility wrappers, lint suppressions, or new abstractions.

## Acceptance Criteria

### B-001 — Production source is unchanged

The existing `src/free.rs:1-379` production prefix remains byte-identical to the fixed base. The parent changes
only by replacing the inline module with `#[cfg(test)] mod tests;`.

### B-002 — Test body is preserved

The existing 45-line inline body moves to `src/free/tests.rs`. The only permitted normalization is removing one
four-space module-nesting indent from each of its 42 nonblank lines; the resulting body otherwise remains exact.

### B-003 — Test contract is preserved

The `free::tests` namespace and the three existing test names, fixture calls, numeric/staleness inputs, comments,
and assertions remain unchanged and occur exactly once.

### B-004 — Scope and size are bounded

Implementation changes exactly `src/free.rs` and adds `src/free/tests.rs`. The parent is below 400 lines and the
child contains only the moved tests; no production visibility or broader module surface is added.

### B-005 — Behavior and adjacent test support do not drift

`src/test_support.rs`, production behavior, public APIs, dependencies, features, workflows, docs, cleanup rules,
and trust-model behavior remain unchanged.

### B-006 — Merge evidence is fresh

Exact forward/rollback proofs, focused stable and exact Rust 1.95.0 tests, full local gates, VibeGuard guards,
independent review, current-head four-platform CI, valid signatures, reviewThreads, and the SpecRail required PR
gate all pass before merge.

## Done When

All B-001 through B-006 criteria are satisfied, the linked implementation PR merges from its verified exact head,
and Issue #334 closes.
