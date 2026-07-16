# GH256 Tasks

## Linked Artifacts

- Issue: `#256`
- Product spec: `specs/GH256/product.md`
- Tech spec: `specs/GH256/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH256 Spec PR to merge.

## Implementation Tasks

### SP256-T1 — Capture the baseline inventory and create the module skeleton

- Owner: `implementation`
- Dependencies: merged GH256 Spec PR; latest `origin/main`
- Covers: B-001, B-003, B-004, B-005, B-006
- Change: Save the normalized 36-name baseline outside the repository, keep `tests/rules.rs`
  as the target entry, add the five specified private modules, and move the three helpers once.
- Done when: the skeleton compiles, helper source search returns one definition per helper, and
  the diff scope matches the planned manifest.
- Verify:
  - `cargo test --test rules --no-run`
  - `rg -n '^fn (make_dir|make_non_empty_path|scan_and_expect_rule)' tests/rules.rs tests/rules`
  - `git diff --name-only origin/main...HEAD`

### SP256-T2 — Mechanically move all 36 tests by domain

- Owner: `implementation`
- Dependencies: SP256-T1
- Covers: B-001, B-002, B-004, B-005, B-007
- Change: Move complete tests into project_artifacts, tool_caches, ai_models, and
  platform_safety without changing bodies; remove only the redundant second Xcode `#[test]`.
- Done when: post-change normalized inventory exactly matches the saved 36-name baseline, every
  test occurs once, and every target file is below 400 lines.
- Verify:
  - `cargo test --test rules -- --list`
  - normalized pre/post `diff -u`
  - `wc -l tests/rules.rs tests/rules/*.rs`
  - `git diff --color-moved=dimmed-zebra origin/main...HEAD`

### SP256-T3 — Verify focused domains and the complete rules target

- Owner: `verification`
- Dependencies: SP256-T2
- Covers: B-001, B-002, B-004, B-007, B-008
- Done when: the full rules target and one representative bare-name filter from each domain pass,
  including the moved blocked-safety case.
- Verify:
  - `cargo test --test rules`
  - `cargo test --test rules rust_target_is_classified`
  - `cargo test --test rules cargo_registry_cache_is_classified_under_cargo_registry`
  - `cargo test --test rules ai_cache_names_outside_exact_anchors_are_not_classified`
  - `cargo test --test rules docker_daemon_storage_candidates_are_blocked`

## Verification And Handoff Tasks

### SP256-T4 — Run the full gate and SpecRail implementation audit

- Owner: `verification`
- Dependencies: SP256-T1, SP256-T2, SP256-T3
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008
- Done when: default/release/MSRV gates pass; VibeGuard adds no finding; non-move hunks are only
  declarations/imports/helper visibility/B-007; current-head PR gate is green.
- Verify:
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95 cargo build --all-targets --all-features`
  - `rustup run 1.95 cargo test`
  - all installed VibeGuard Rust guards

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008}`
- Missing invariants: `none`

## Handoff Notes

- Do not modify production code, dependencies, CI, docs, fixtures, assertions, or test names.
- Implementation begins from the merged Spec PR on the latest `origin/main`.
- Merge only with fresh current-head gates under the standing authorization; never force push.
