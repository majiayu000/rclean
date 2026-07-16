# Feature-gated Audit Variant Warning Cleanup - Tech Spec

## Linked Artifacts

- GitHub issue: `#290`
- Product spec: `specs/GH290/product.md`
- Tasks: `specs/GH290/tasks.md`
- Route: `write_spec`

## Root Cause Evidence

| Area | Evidence | Decision |
| --- | --- | --- |
| `src/clean/audit.rs` | `Graveyard`/`Skipped` variants unconditional | Gate each exact variant with `graveyard`. |
| `src/clean/deletion.rs` | all construction sites already inside `#[cfg(feature = "graveyard")]` | Do not modify deletion code. |
| CI feature matrix | ordinary tests pass but do not deny warnings | Add explicit local/PR evidence with `RUSTFLAGS=-D warnings`. |
| serde contract | enum-level `rename_all = "snake_case"` | Add focused serialization tests for exact strings. |

Fresh baseline on `origin/main@eebf92a` exits 101 with exactly two `dead_code` errors under
`RUSTFLAGS='-D warnings' cargo test --no-default-features --no-run`.

## Design

Apply the same existing feature boundary directly to the two variants:

```rust
pub enum DeleteAuditMode {
    Trash,
    Permanent,
    #[cfg(feature = "graveyard")]
    Graveyard,
    GoModcache,
    PipCache,
}

pub enum DeleteAuditStatus {
    Success,
    Failed,
    #[cfg(feature = "graveyard")]
    Skipped,
}
```

No facade, helper, alias, dummy use or lint attribute is introduced. Because every production
construction site already has the same feature cfg, feature-enabled behavior remains unchanged and
feature-disabled builds no longer carry unreachable variants.

## Test Design

Add private unit tests in `src/clean/audit.rs`:

- a base table asserts exact serde JSON strings for all always-available modes/statuses;
- `#[cfg(feature = "graveyard")]` assertions cover `Graveyard` and `Skipped`;
- the no-default/tui-only builds prove gated names are absent at compile time and warning-clean;
- existing graveyard CLI/audit tests prove the emitted audit path still works end to end.

The tests must use `serde_json::to_string`; substring assertions on larger audit output are not a
replacement for the enum contract.

## Product-to-Test Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001 gated absence | variant cfg | no-default and tui-only warnings-as-errors builds |
| B-002 graveyard names | feature-gated variants | focused serde assertions + graveyard-only build |
| B-003 base names | unchanged variants | table-driven serde assertions in every feature build |
| B-004 runtime compatibility | no deletion-code changes | existing `graveyard_cli` and full tests |
| B-005 no suppression | source/diff inspection | `rg 'allow\(dead_code\)|expect\(dead_code\)'` in diff is empty |
| B-006 exact scope | one-file manifest | `git diff --name-only origin/main...HEAD` |
| B-007 full matrix | build/test gates | stable/MSRV/CI/VibeGuard commands |

## Planned Change Manifest

| Path | Change |
| --- | --- |
| `src/clean/audit.rs` | Feature-gate two variants and add focused serialization contract tests. |

No other source, test, dependency, schema, workflow, documentation or private advisory artifact is
in scope.

## Risk Analysis

- **Serialized compatibility:** guarded by exact string tests; enabled variants keep the same Rust
  and serde names.
- **Feature drift:** guarded by four warnings-as-errors feature builds plus current CI matrix.
- **Deletion behavior:** no call sites, matching, selection or audit logging flow changes.
- **Schema drift:** enum values under enabled features remain identical; disabled features cannot
  emit values whose behavior is not compiled.

## Verification Plan

```sh
git diff --check
git diff --name-only origin/main...HEAD
cargo test clean::audit::tests
RUSTFLAGS='-D warnings' cargo test --no-default-features --no-run
RUSTFLAGS='-D warnings' cargo test --no-default-features --features tui --no-run
RUSTFLAGS='-D warnings' cargo test --no-default-features --features graveyard --no-run
RUSTFLAGS='-D warnings' cargo test --all-features --no-run
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

## Rollback

Revert the implementation commit. Feature-disabled builds resume emitting the two known warnings;
there is no data, migration, dependency or runtime state to roll back.

## Human Gates

- Spec and implementation remain separate PRs.
- This change does not alter deletion or graveyard runtime behavior; any such drift stops the task.
- Merge only after current-head CI, zero unresolved review threads and clean merge state under the
  user's standing authorization; never force push.
