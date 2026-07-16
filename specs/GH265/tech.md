# Doctor Global Catalog Coverage - Tech Spec

## Linked Artifacts

- GitHub issue: `#265`
- Product spec: `specs/GH265/product.md`
- Tasks: `specs/GH265/tasks.md`
- Route: `write_spec`

## Codebase Context

| Area | Evidence | Decision |
| --- | --- | --- |
| `src/doctor.rs` | 744 lines; production ends before inline tests at line 652 | Externalize child test module without production edits. |
| count-only test | asserts only `total_count() == 59` | Replace with exact catalog-derived set equality. |
| `rules::rule_catalog` + `rules::is_global_rule` | existing catalog/safety sources classify the current 59 global IDs | Derive expected IDs; do not add a third list. |

## Proposed Design

Replace the inline module with:

```rust
#[cfg(test)]
mod tests;
```

Move its body to `src/doctor/tests.rs`. Keep `use super::*` and the existing HOME serialization
guard. Replace only the first test with a test named
`diagnose_matches_global_rule_catalog_exactly`.

The new test builds two `BTreeSet<&'static str>` values:

- expected: `crate::rules::rule_catalog()`, filtered by
  `crate::rules::is_global_rule(rule.rule_id)`, mapped to `rule_id`;
- actual: `diagnose().entries`, mapped to `entry.rule_id`.

First assert `report.entries.len() == actual.len()` to reject duplicates. Then assert actual equals
expected so assertion output exposes missing/extra IDs. Do not retain a fixed numeric count.

## Product-to-Test Mapping

| Invariant | Evidence |
| --- | --- |
| B-001 production unchanged | pre/post parent prefix diff and `wc -l` |
| B-002 exact identity | `diagnose_matches_global_rule_catalog_exactly` set equality |
| B-003 duplicate rejection | entries length equals actual set length |
| B-004 no magic number | implementation diff contains no `total_count() == 59` or replacement count |
| B-005 state tests unchanged | mechanical body comparison plus focused pass |
| B-006 scope | `git diff --name-only origin/main...HEAD` equals two paths; no production visibility diff |
| B-007 gates | focused/full/MSRV/VibeGuard/current-head CI evidence |

## Planned Changes Manifest

| Path | Change |
| --- | --- |
| `src/doctor.rs` | Replace inline tests with external child-module declaration; no production edit. |
| `src/doctor/tests.rs` | Move helpers/tests; replace only count test with exact catalog and duplicate assertions. |

No other implementation path is permitted.

## Verification Plan

```sh
cargo test doctor::tests
wc -l src/doctor.rs src/doctor/tests.rs
git diff --check
git diff --name-only origin/main...HEAD
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

Run all installed VibeGuard Rust guards and require Ubuntu/macOS/Windows/MSRV CI success for the
current head SHA.

## Risks And Mitigations

- **Circular self-fulfilling list:** expected IDs come from existing catalog + safety predicate,
  not doctor source.
- **Duplicate hidden by set:** separate length assertion rejects duplicates before equality.
- **Platform mismatch:** doctor already emits skipped entries for unsupported platform rules; run
  three-platform CI.
- **HOME race:** retain the existing serialized HOME guard unchanged.
- **Scope creep:** reject any production/catalog/rule change.

## Rollback

Revert the implementation commit. No runtime, schema, persistence, or dependency state exists.

## Human Gates

- Spec and implementation remain separate PRs.
- Merge only after current-head CI, unresolved-thread, merge-state, structural and set-coverage
  evidence are green.
- The user has provided standing merge authorization; never force push.
