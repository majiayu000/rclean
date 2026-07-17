# GH326 Product Spec

## Linked Issue

- `#326` — `refactor(test): centralize ranking report fixtures`

## Problem

The `free` and `output` ranking unit tests each own the same model-construction knowledge. Their 35-line
`report_with` functions are byte-identical, while their 16-line `candidate` functions differ only because the
`free` tests pass `Safety` explicitly and the `output` tests hard-code `Safety::Safe`.

That duplication makes routine model evolution error-prone: adding a required `Candidate`, `Summary`,
`ProjectReport`, or `ScanReport` field requires synchronized edits in two sibling modules even though both suites
exercise the same ranking input shape.

## Goals

1. Give the two ranking suites one typed source of truth for candidate and report fixtures.
2. Preserve the semantic inputs and assertions of all existing `free` and `output` unit tests.
3. Keep the shared fixture code outside production and release builds.
4. Reduce duplicated schema construction without introducing a macro, dependency, or production abstraction.

## Non-Goals

- Changing staleness ranking, free-target selection, formatting, scan output, or cleanup behavior.
- Changing any test name, assertion, expected value, safety case, or production function.
- Sharing fixtures with unrelated plan, TUI, watch, scan, or integration-test suites.
- Adding public APIs, compatibility aliases, macros, dependencies, retries, skips, or feature flags.
- Touching ActionPlan, delete modes, broad-root, symlink, protected-path, or security behavior.

## Acceptance Criteria

### B-001 — One test-only source of truth

Exactly one crate-private module compiled only under `cfg(test)` owns the common ranking candidate and report
constructors. The constructors remain typed Rust functions, not macros.

### B-002 — Free safety coverage is preserved

The three `free::tests` keep their names, numeric/staleness inputs, assertions, and explicit use of `Safe`,
`Caution`, `Blocked`, and `ReportOnly`. The shared candidate constructor accepts `Safety` as an explicit argument.

### B-003 — Output safety semantics are explicit

The three `output::tests` keep their names, numeric/staleness inputs, assertions, and formatting expectations.
Every output ranking candidate explicitly supplies `Safety::Safe` to the shared constructor.

### B-004 — Duplicate local fixture knowledge is removed

`src/free.rs` and `src/output.rs` no longer define local candidate or report constructors. The byte-identical
report schema is represented once, and no compatibility alias or second wrapper recreates the duplication.

### B-005 — Production behavior and build graph are unchanged

No production function, dependency, feature, workflow, or user-visible contract changes. The new module is
declared behind `#[cfg(test)]`, and release builds remain successful without compiling it into the production
binary.

### B-006 — Scope remains bounded and maintainable

Implementation changes only `src/main.rs`, `src/free.rs`, `src/output.rs`, and one new test-support source file.
Every Rust file remains below the 800-line ceiling, and the total duplicated fixture lines decrease.

### B-007 — Verification and merge evidence are fresh

Focused stable and Rust 1.95.0 tests, full stable/MSRV gates, VibeGuard guards, exact diff/source audits,
independent review, current-head cross-platform CI, signatures, reviewThreads, and the SpecRail required PR gate
all pass before merge.

## Done When

All B-001 through B-007 criteria are satisfied, the linked implementation PR merges from its verified exact head,
and Issue #326 closes.
