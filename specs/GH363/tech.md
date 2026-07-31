# Fail-Closed ActionPlan Replay Sizing - Tech Spec

## Linked Artifacts

- GitHub pull request: `#363`
- Product spec: `specs/GH363/product.md`
- Tasks: `specs/GH363/tasks.md`
- Route after maintainer gate: `implement`

```specrail-planned-changes
{
  "pr": 363,
  "complete": true,
  "paths": [
    "specs/GH363/product.md",
    "specs/GH363/tech.md",
    "specs/GH363/tasks.md",
    "src/plan/revalidate.rs",
    "src/plan/tests.rs",
    "src/scan/sizer.rs",
    "src/scan/sizer/tests.rs"
  ],
  "spec_refs": [
    "specs/GH363/product.md",
    "specs/GH363/tech.md",
    "specs/GH363/tasks.md"
  ]
}
```

## Current Data Flow

```text
dir_size(path)
  -> SizeOutcome { bytes, newest_mtime, warnings }

candidate_dir_size_bytes(path)
  -> outcome.bytes

revalidate_selected
  -> candidate.bytes = partial bytes
  -> selected candidate reaches confirmation/deletion
```

The scan sizer intentionally preserves readable bytes after an entry fails and
records completeness in `SizeOutcome.warnings`. The new replay adapter erases
that completeness signal.

## Design

### 1. Make the replay adapter fallible

Keep `SizeOutcome` private and change the PR-local crate-visible adapter:

```rust
pub(crate) fn candidate_dir_size_bytes(
    path: &Path,
) -> Result<u64, Vec<ScanWarning>>
```

Implementation:

1. call the existing `dir_size(path, false)`;
2. return `Ok(outcome.bytes)` only when `outcome.warnings.is_empty()`;
3. otherwise return every warning in `Err(outcome.warnings)`.

This does not change `summarize`, `dir_size`, warning ordering, parallel
walking, or ordinary scan output.

### 2. Convert warnings at the plan boundary

In `revalidate_selected`, map the warning vector into a contextual
`PlanError::Generic` before assigning `candidate.bytes`:

```text
failed to determine current size for <candidate>: <warning 1>; <warning 2>
```

The candidate path is included even if an individual walk error lacks a path.
Every structured warning is formatted through its existing `Display`
implementation and retained in deterministic sizer order.

The existing `?` in the clean workflow propagates `PlanError`, so confirmation,
deletion, audit persistence, and graveyard persistence are not reached.

### 3. Privilege-independent regression fixtures

Use two deterministic layers instead of relying on `chmod 0o000`, which root
or a process with `CAP_DAC_OVERRIDE` can still read:

1. In `src/scan/sizer/tests.rs`, call the real replay adapter with a missing
   path and assert that it returns the resulting `MetadataError`, not `Ok(0)`.
2. Route public `revalidate_selected` through a crate-internal generic helper
   that accepts the current-size function. Production passes
   `candidate_dir_size_bytes`.
3. In `src/plan/tests.rs`, pass a test closure returning two structured
   warnings (`WalkError` and `MetadataError`) with distinct paths.
4. Assert the plan error includes the candidate, both paths, and both error
   messages.

The injected function changes no public API or production behavior. It makes
the plan-boundary contract deterministic on Unix, Windows, root containers,
and capability-enabled build environments.

## Error Contract

| Source | Scan behavior | ActionPlan replay behavior |
| --- | --- | --- |
| `WalkError` | partial bytes plus warning | fatal `PlanError` |
| `MetadataError` | partial bytes plus warning | fatal `PlanError` |
| no warnings | complete bytes | update selected candidate |

No warning is logged and ignored. No stale-byte fallback exists.

## Files Touched

| File | Change |
| --- | --- |
| `src/scan/sizer.rs` | Return a strict result from the replay-only adapter. |
| `src/scan/sizer/tests.rs` | Prove the real adapter rejects deterministic metadata failure. |
| `src/plan/revalidate.rs` | Convert all sizing warnings to contextual `PlanError`. |
| `src/plan/tests.rs` | Inject structured sizing failures and prove complete propagation. |
| `specs/GH363/*` | Record product, technical, task, and verification contracts. |

## Forbidden Scope

- `AGENTS.md`
- `SECURITY.md`
- ActionPlan schema modules
- deletion, graveyard, or restore implementations
- `docs/audit-2026-05-07.md`

If the implementation requires any forbidden path, stop and reopen the
maintainer gate.

## Product-to-Test Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001 | fallible adapter success branch | existing stale-byte update test |
| B-002 | warning vector maps to `PlanError` | injected plan regression |
| B-003 | contextual join of all warnings | two injected paths/messages + sizer warning ordering tests |
| B-004 | no fallback branch | code inspection + regression fails before deletion |
| B-005 | `summarize` unchanged | existing scan/sizer test suite |
| B-006 | no trust-gate changes | existing plan and safety tests + full suite |

## Verification

Focused RED/GREEN:

```sh
cargo test plan::tests::revalidation_rejects_incomplete_current_size -- --exact
cargo test plan::tests::revalidation_updates_stale_bytes_from_disk -- --exact
cargo test scan::sizer::tests
```

Full gate:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95 cargo build --all-targets --all-features
rustup run 1.95 cargo test
```

## Risks And Mitigations

- **Ordinary scan regression:** the strict result exists only at the replay
  adapter; scan summarizing remains unchanged.
- **Lost diagnostics:** the error joins the complete warning vector.
- **Test seam drift:** the injected function is crate-internal, while the
  public production wrapper always supplies the real replay adapter; both
  layers have direct tests.
- **Scope drift:** planned paths and forbidden paths are machine-readable and
  reviewed before commit.
