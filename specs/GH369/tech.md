# GH369 Fail-Closed Safety Corrections - Tech Spec

## Linked Artifacts

- GitHub issue: `#369`
- Issue URL: `https://github.com/majiayu000/rclean/issues/369`
- Implementation history: PR `#363`
- Product spec: `specs/GH369/product.md`
- Tasks: `specs/GH369/tasks.md`
- Route: `plan_first`

```specrail-planned-changes
{
  "issue": 369,
  "implementation_pr": 363,
  "complete": true,
  "paths": [
    "specs/GH369/product.md",
    "specs/GH369/tech.md",
    "specs/GH369/tasks.md",
    "src/main.rs",
    "src/plan/revalidate.rs",
    "src/plan/tests.rs",
    "src/scan/mod.rs",
    "src/scan/safety.rs",
    "src/scan/sizer.rs",
    "src/scan/sizer/tests.rs"
  ],
  "spec_refs": [
    "specs/GH369/product.md",
    "specs/GH369/tech.md",
    "specs/GH369/tasks.md"
  ]
}
```

## Shipped H5 Design — Fail-Closed Containment

For a non-dot scan root, `apply_path_safety` evaluates the root and candidate
canonicalization results together:

```text
(canonical root, canonical candidate)
  -> candidate outside root: Safety::Blocked + containment warning
  -> either canonicalization fails: Safety::Blocked + failure warning
  -> otherwise: preserve the established safety classification
```

The canonicalization-failure warning begins:

```text
failed to canonicalize path for containment check:
```

The function returns early only for the pre-existing higher-priority safety
gates. This change removed the former `.canonicalize().ok()` path that silently
skipped containment when either conversion failed.

Focused regressions in `src/scan/safety.rs` cover both branches:

- `canonicalize_failure_blocks_candidate`
- `candidate_resolving_outside_root_is_blocked`

## Shipped H8 Design — Strict Replay Sizing

### Prior data flow

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
records completeness in `SizeOutcome.warnings`. The initial replay adapter
erased that completeness signal.

### Fallible replay adapter

Keep `SizeOutcome` private and expose a crate-visible replay adapter:

```rust
pub(crate) fn candidate_dir_size_bytes(
    path: &Path,
) -> Result<u64, Vec<ScanWarning>>
```

The adapter:

1. calls the existing `dir_size(path, false)`;
2. returns `Ok(outcome.bytes)` only when `outcome.warnings.is_empty()`;
3. otherwise returns every warning in `Err(outcome.warnings)`.

This does not change `summarize`, `dir_size`, warning ordering, parallel
walking, or ordinary scan output.

### Plan-boundary conversion

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

### Privilege-independent regression fixtures

The implementation uses two deterministic layers rather than relying on
`chmod 0o000`, which root or a process with `CAP_DAC_OVERRIDE` can still read:

1. `src/scan/sizer/tests.rs` calls the real replay adapter with a missing path
   and asserts that it returns the resulting `MetadataError`, not `Ok(0)`.
2. Public `revalidate_selected` routes through a crate-internal generic helper
   that accepts the current-size function. Production supplies
   `candidate_dir_size_bytes`.
3. `src/plan/tests.rs` supplies a closure returning both `WalkError` and
   `MetadataError` with distinct paths.
4. The plan test asserts that the error includes the candidate, both paths, and
   both error messages.

The seam changes no public API or production behavior and stays deterministic
on Unix, Windows, root containers, and capability-enabled build environments.

## Error Contract

| Source | Scan behavior | ActionPlan replay behavior |
| --- | --- | --- |
| `WalkError` | partial bytes plus warning | fatal `PlanError` |
| `MetadataError` | partial bytes plus warning | fatal `PlanError` |
| no warnings | complete bytes | update selected candidate |

No warning is logged and ignored. No stale-byte fallback exists.

## Complete Shipped Scope

| File | Change |
| --- | --- |
| `src/scan/safety.rs` | Block canonicalization failure or canonical escape and test both branches. |
| `src/scan/mod.rs` | Export the strict replay-sizing adapter to the plan layer. |
| `src/main.rs` | Consume the selected candidates returned after revalidation. |
| `src/scan/sizer.rs` | Return a strict result from the replay-only adapter. |
| `src/scan/sizer/tests.rs` | Prove the real adapter rejects deterministic metadata failure. |
| `src/plan/revalidate.rs` | Convert all sizing warnings to contextual `PlanError`. |
| `src/plan/tests.rs` | Inject structured sizing failures and prove complete propagation. |
| `specs/GH369/*` | Record the complete product, technical, task, and verification contracts. |

## Forbidden Scope For Issue #369

- production and test code
- `AGENTS.md`
- `SECURITY.md`
- ActionPlan schema modules
- deletion, graveyard, or restore implementations
- `docs/audit-2026-05-07.md`

## Product-to-Test Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001 | `apply_path_safety` canonicalization error branch | `canonicalize_failure_blocks_candidate` |
| B-002 | `apply_path_safety` outside-root branch | `candidate_resolving_outside_root_is_blocked` |
| B-003 | fallible adapter success branch | stale-byte update test |
| B-004 | warning vector maps to `PlanError` | injected plan regression |
| B-005 | contextual join of all warnings | two injected paths/messages + sizer warning ordering tests |
| B-006 | no fallback branch | code inspection + regression fails before deletion |
| B-007 | `summarize` unchanged | existing scan/sizer test suite |
| B-008 | no unrelated trust-gate changes | existing plan and safety tests + full suite |

## Historical Implementation Verification

Focused RED/GREEN:

```sh
cargo test scan::safety::tests::canonicalize_failure_blocks_candidate -- --exact
cargo test scan::safety::tests::candidate_resolving_outside_root_is_blocked -- --exact
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

Issue #369 changes documentation only. Its focused verification is
`git diff --check`, `cargo fmt -- --check`, exact packet counts, link checks,
and confirmation that no Rust or runtime file changed.

## Risks And Mitigations

- **Incomplete safety history:** H5 implementation paths and focused tests are
  explicitly mapped alongside H8.
- **Ordinary scan regression:** the strict result exists only at the replay
  adapter; scan summarizing remains unchanged.
- **Lost diagnostics:** the error joins the complete warning vector.
- **Test seam drift:** the injected function is crate-internal, while the
  public production wrapper always supplies the real replay adapter; both
  layers have direct tests.
- **Documentation scope drift:** issue #369 moves the existing packet and
  updates the retained index without changing shipped production or test code.
