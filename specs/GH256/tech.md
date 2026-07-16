# Rule Integration Test Suite Split - Tech Spec

## Linked Artifacts

- GitHub issue: `#256`
- Product spec: `specs/GH256/product.md`
- Tasks: `specs/GH256/tasks.md`
- Route: `write_spec`

## Codebase Context

| Area | Current evidence | Decision |
| --- | --- | --- |
| `tests/rules.rs:1-38` | target-level imports and three shared fixture helpers | Keep the root as target entry; move helpers to `rules/common.rs`. |
| `tests/rules.rs:40-190` | project artifact and ambiguous build dispatch tests | Move to `project_artifacts.rs`. |
| `tests/rules.rs:192-273,680-767` | Xcode positive/clean/plan/negative tests and Docker blocked safety test | Move to `platform_safety.rs`. |
| `tests/rules.rs:278-417,522-679,702-719` | Cargo, Homebrew, Dart, Node, pip, Gradle and Maven cache tests | Move to `tool_caches.rs`. |
| `tests/rules.rs:418-520` | AI cache/model-store tests | Move to `ai_models.rs`. |
| `tests/rules.rs:218-220` | duplicate `#[test]` before one Xcode test | Remove only the redundant second attribute while preserving one registration. |

## Proposed Module Layout

```text
tests/
├── rules.rs                    # only private module declarations; target remains rules
└── rules/
    ├── common.rs               # three shared fixture helpers
    ├── project_artifacts.rs    # project-local build/cache classification and priority
    ├── tool_caches.rs          # package-manager and toolchain global caches
    ├── ai_models.rs            # AI cache/model-store boundaries
    └── platform_safety.rs      # Xcode/platform paths and blocked Docker safety
```

All modules use normal `mod` declarations. `common` exposes helpers as `pub(super)` only.
Each test module imports its own external crates and `std` items, then imports only the helpers
it uses. No helper body or test body is rewritten.

## Mechanical Move Rules

1. Capture the pre-change test list and normalize full paths to bare function names.
2. Create the root module declarations and minimal module-local imports.
3. Move complete attribute + test items into the mapped domain module; move helpers once.
4. Preserve function names and bodies byte-for-byte where possible. The only allowed removed
   line outside imports/root wiring is the redundant B-007 `#[test]` attribute.
5. Compare sorted pre/post bare-name inventories and require an empty diff plus count 36.
6. Review `git diff --color-moved=dimmed-zebra` so non-move hunks are limited to module
   declarations, imports, helper visibility, and the redundant attribute removal.

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 exact 36-name inventory | all modules | pre/post normalized `cargo test --test rules -- --list`, `diff -u`, unique count 36 |
| B-002 behavior unchanged | moved test items | moved-code review plus complete focused/full suites |
| B-003 one helper source | `tests/rules/common.rs` | `rg -n '^fn (make_dir|make_non_empty_path|scan_and_expect_rule)' tests/rules.rs tests/rules` returns one each |
| B-004 target/filter compatibility | `tests/rules.rs` and module paths | representative bare-name filters for all four domains |
| B-005 files below 400 lines | proposed module layout | `wc -l tests/rules.rs tests/rules/*.rs` |
| B-006 test-only scope | full diff | changed path allowlist contains only `tests/rules.rs` and `tests/rules/*.rs` |
| B-007 redundant attribute only | `platform_safety.rs` | list uniqueness plus diff review of the affected test |
| B-008 supported gates | unchanged target under CI | focused/default/release/MSRV checks and GitHub CI |

## Planned Changes Manifest

| Path | Change |
| --- | --- |
| `tests/rules.rs` | Replace monolithic contents with five private module declarations. |
| `tests/rules/common.rs` | Add the three existing shared fixture helpers with target-private visibility. |
| `tests/rules/project_artifacts.rs` | Move project-local artifact and dispatch-priority tests. |
| `tests/rules/tool_caches.rs` | Move global package-manager/tool cache tests. |
| `tests/rules/ai_models.rs` | Move AI cache/model-store tests. |
| `tests/rules/platform_safety.rs` | Move Xcode/platform and Docker blocked-safety tests; remove one redundant attribute. |

No other path is permitted in the implementation PR.

## Dependencies And Ordering

- The Spec PR contains only `specs/GH256/` and can merge independently.
- Implementation starts from the latest `origin/main` after the Spec PR merges.
- PR #235 is unrelated; this refactor must not update dependencies or absorb that diff.

## Risks And Mitigations

- **Risk:** A test or attribute is lost during movement. **Mitigation:** exact pre/post normalized
  name-set diff, count 36, and moved-code review.
- **Risk:** Module imports cause test-body edits. **Mitigation:** allow non-move hunks only for
  imports, declarations, helper visibility, and B-007.
- **Risk:** Shared fixtures are copied into domain modules. **Mitigation:** common module plus
  deterministic three-name source search.
- **Risk:** A filter consumer depends on bare names. **Mitigation:** function names remain stable
  and representative bare-name filters are run.
- **Risk:** Safety assertions drift during reorganization. **Mitigation:** no production change,
  unchanged test bodies, focused platform/safety filters, and full CI.

## Verification Plan

Baseline and focused:

```sh
cargo test --test rules -- --list
cargo test --test rules
cargo test --test rules rust_target_is_classified
cargo test --test rules cargo_registry_cache_is_classified_under_cargo_registry
cargo test --test rules ai_cache_names_outside_exact_anchors_are_not_classified
cargo test --test rules docker_daemon_storage_candidates_are_blocked
```

Structure and scope:

```sh
wc -l tests/rules.rs tests/rules/*.rs
rg -n '^fn (make_dir|make_non_empty_path|scan_and_expect_rule)' tests/rules.rs tests/rules
git diff --color-moved=dimmed-zebra origin/main...HEAD
git diff --check
git diff --name-only origin/main...HEAD
```

Repository gate:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95 cargo build --all-targets --all-features
rustup run 1.95 cargo test
```

## Rollback

The implementation only moves test code. Revert its single implementation commit if discovery,
filtering, or assertions change; there is no production, schema, or data migration.

## Human Gates

- Spec and implementation remain separate PRs.
- Merge only after current-head CI, review-thread, merge-state, and scope evidence is green.
- The user has provided standing merge authorization for this optimization run; never force push.
