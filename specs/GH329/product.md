# GH329 Product Spec

## Linked Issue

- `#329` — `refactor(doctor): split applicability entry construction`

## Problem

`src/doctor.rs` is 653 lines and its tests are already isolated. The remaining production file is dominated by
the 514-line `diagnose_with_options` constructor, which combines public report orchestration with 59 ordered
shared and platform-specific applicability entries.

This puts unrelated review responsibilities in one file and leaves the module above the repository's 200-400
line typical range. The current output is correct and covered by exact catalog-identity tests, so the work is a
behavior-neutral maintainability refactor rather than a behavior fix.

## Goals

1. Separate shared and platform-specific doctor entry construction behind private child modules.
2. Keep the public doctor report/API orchestration small and explicit.
3. Preserve every existing rule ID, order, anchor, status, reason, cfg branch, and Docker opt-in behavior.
4. Bring each affected production file below 400 lines without adding a new registry or abstraction layer.

## Non-Goals

- Adding, removing, reclassifying, or renaming cleanup rules.
- Changing doctor output, scan reachability, cleanup selection, deletion, ActionPlan, Docker timeout, or safety.
- Changing tests, assertions, documentation, dependencies, workflows, features, or public APIs.
- Converting entries to a data registry, macro, trait, builder, alias, or compatibility wrapper.
- Touching broad-root, symlink, protected-path, graveyard, restore, or security behavior.

## Acceptance Criteria

### B-001 — Private responsibility boundaries

Doctor entry construction is split into exactly two new private child modules: one returns the existing ordered
common prefix and one appends the existing ordered platform-specific suffix. No public or crate-public item is
added.

### B-002 — Common entries are preserved

The common module preserves the current 21-entry initial vector, cfg-specific pnpm/pip/Go anchors, and subsequent
AI, Python, Deno, browser, JetBrains, and Android Studio entries in the same order with the same values.

### B-003 — Platform entries are preserved

The platform module preserves the current macOS entries and every Linux/Windows skipped fallback in the same
order, including exact paths, statuses, reason strings, and cfg predicates.

### B-004 — Public orchestration is preserved

`diagnose` and `diagnose_with_options` retain their signatures. Missing `HOME` still produces an empty report;
common entries precede platform entries; `include_docker` still appends exactly one Docker entry last using the
unchanged five-second probe timeout.

### B-005 — Doctor output and APIs do not drift

Existing doctor rule identities, entry order, anchors, statuses, reasons, and serialization consumers remain
unchanged. No rule/catalog, public API, visibility, dependency, feature, or user-facing behavior changes.

### B-006 — Scope and module sizes are bounded

Implementation changes only `src/doctor.rs` and adds `src/doctor/common_entries.rs` plus
`src/doctor/platform_entries.rs`. Each affected production file is below 400 lines; tests and `anchors.rs` remain
unchanged.

### B-007 — Existing test contract is unchanged

`src/doctor/tests.rs` remains byte-identical, its three test names remain present exactly once, and focused stable
and exact Rust 1.95.0 doctor tests pass.

### B-008 — Merge evidence is fresh

Exact extraction/reconstruction proofs, fmt, clippy, full stable/release and Rust 1.95.0 gates, VibeGuard guards,
independent review, current-head cross-platform CI, signatures, reviewThreads, and the SpecRail required PR gate
all pass before merge.

## Done When

All B-001 through B-008 criteria are satisfied, the linked implementation PR merges from its verified exact head,
and Issue #329 closes.
