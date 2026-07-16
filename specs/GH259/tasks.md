# GH259 Tasks

## Linked Artifacts

- Issue: `#259`
- Product spec: `specs/GH259/product.md`
- Tech spec: `specs/GH259/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH259 Spec PR to merge.

## Implementation Tasks

### SP259-T1 — Add deterministic marker-hint seams and regression fixtures

- Owner: `implementation`
- Dependencies: merged GH259 Spec PR; latest `origin/main`
- Covers: B-001, B-002, B-003, B-007, B-008, B-009
- Change: add private/test-only metadata and Git-program seams; write focused tests for sibling
  no-marker reuse, child directory/file marker priority, environment override, metadata error,
  poison fallback and new-cache isolation.
- Done when: tests deterministically distinguish zero Git invocations from command fallback without
  global `PATH` or process-environment mutation.
- Verify:
  - `cargo test scan::git_cache::tests`
  - focused fake-command invocation assertions

### SP259-T2 — Implement child-first scan-local ancestor marker cache

- Owner: `implementation`
- Dependencies: SP259-T1
- Covers: B-001, B-002, B-003, B-005, B-006, B-007, B-008, B-009
- Change: wire the private marker cache into `GitCache::info_for`; cache only unambiguous results,
  probe child before parent reuse, and route Found/Fallback through unchanged Git commands.
- Done when: non-repo siblings skip commands while parent/nested/file-marker repositories resolve
  through Git and ambiguous/poison states fall back.
- Verify:
  - `cargo test scan::git_cache::tests`
  - `cargo test scan::tests::git_cache_shares_dirty_flag_across_sibling_projects`

### SP259-T3 — Prove safety and report equivalence

- Owner: `verification`
- Dependencies: SP259-T2
- Covers: B-004, B-005, B-006, B-008, B-010
- Done when: existing dirty-git demotion tests pass, parent/nested repo tests pass, and normalized
  before/after JSON from the same 100-sibling fixture has an empty diff.
- Verify:
  - `cargo test scan::tests::dirty_git_marks_candidate_caution`
  - `cargo test scan::tests::git_cache_shares_dirty_flag_across_sibling_projects`
  - normalized JSON comparison from the fixed fixture

### SP259-T4 — Measure same-session performance

- Owner: `verification`
- Dependencies: SP259-T2
- Covers: B-001, B-010
- Done when: at least three warmed release runs per revision show median speedup >=5x for the fixed
  100-sibling fixture; Criterion many-small and one-huge results are recorded without regression.
- Verify:
  - release binary timing table for `origin/main` and implementation
  - `cargo bench --bench scan_throughput -- --noplot`

## Verification And Handoff Tasks

### SP259-T5 — Full gate, VibeGuard and SpecRail audit

- Owner: `verification`
- Dependencies: SP259-T1, SP259-T2, SP259-T3, SP259-T4
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010
- Done when: scope is only `src/scan/git_cache.rs`; full/default/release/MSRV/VibeGuard and
  current-head PR gates pass; spec-vs-implementation has no missing invariant.
- Verify:
  - `git diff --check`
  - `git diff --name-only origin/main...HEAD`
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - all installed VibeGuard Rust guards

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010}`
- Missing invariants: `none`

## Handoff Notes

- Do not change Git root/dirty authority, CLI output, safety classification or failure semantics.
- Do not modify benchmark, existing scan tests, dependencies, CI, docs, or any file outside the
  planned manifest.
- Implementation starts from the merged Spec PR on latest `origin/main`.
- Merge only with fresh current-head gates under the standing authorization; never force push.
