# Watch ActionPlan Collision Prevention - Tech Spec

## Linked Artifacts

- GitHub issue: `#293`
- Product spec: `specs/GH293/product.md`
- Tasks: `specs/GH293/tasks.md`
- Route: `write_spec`

## Root Cause Evidence

| Area | Evidence | Decision |
| --- | --- | --- |
| `timestamped_path` | formats only UTC seconds | Keep the readable prefix, add collision selection outside formatting. |
| `write_timestamped_plan` | calls the atomic writer for every refresh | Resolve a unique unused path immediately before the existing writer. |
| runtime reproduction | 49 messages, 2 files, one path written 48 times | Add deterministic same-stamp regression coverage. |
| ActionPlan I/O | existing writer may replace its target atomically | Do not change shared plan I/O or replay; prevent known paths from reaching it. |

## Design

Format `Utc::now()` exactly once per refresh, then call a private helper conceptually equivalent to:

```rust
fn next_timestamped_path(base: &Path, stamp: &str) -> Result<PathBuf, PlanError> {
    let first = timestamped_path(base, stamp, None);
    if !first.try_exists().map_err(|source| PlanError::Io {
        path: first.clone(),
        source,
    })? {
        return Ok(first);
    }

    let mut sequence = 2_u64;
    loop {
        let candidate = timestamped_path(base, stamp, Some(sequence));
        if !candidate.try_exists().map_err(|source| PlanError::Io {
            path: candidate.clone(),
            source,
        })? {
            return Ok(candidate);
        }
        sequence = sequence.checked_add(1).ok_or_else(|| {
            PlanError::Generic("watch plan collision sequence exhausted".to_string())
        })?;
    }
}
```

The pure formatter receives a fixed `stamp` and optional sequence, so tests do not depend on clock
precision. The production caller formats `Utc::now()` once, resolves the path, calls the unchanged
`plan::write_action_plan`, and prints success only after the write returns `Ok`.

This is a serial single-process guarantee. A cross-process create-new reservation would require a
shared ActionPlan writer contract change and is explicitly outside this issue.

## Test Design

Private tests in `src/watch/mod.rs` will:

- use a fixed stamp and empty temp directory to assert the current unsuffixed name;
- create a sentinel first path, then assert `-2` without changing sentinel bytes;
- create the `-2` path and assert `-3`;
- cover a base without an extension;
- retain the platform-gated non-UTF-8 fallback test where supported;
- inject or isolate existence probing so an I/O error and counter exhaustion are deterministic and
  explicit rather than requiring filesystem races.

The regression test must be observed failing against the old single-path selection before the fix,
then pass after collision probing is implemented. Existing watch state and CLI tests stay unchanged.

## Product-to-Test Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001 first path | pure formatter + initial probe | fixed-stamp first-choice test |
| B-002 suffixes/no overwrite | collision loop | sentinel `-2`/`-3` tests |
| B-003 one success per path | resolve before writer | focused helper test + existing write sequencing review |
| B-004 name compatibility | stem/extension formatter | JSON, extensionless, non-UTF-8 tests |
| B-005 explicit errors | `try_exists` + checked counter | injected error/exhaustion tests |
| B-006 unchanged plan/watch behavior | existing writer/call graph | scoped diff + full tests |
| B-007 full gate | one-file manifest | stable/MSRV/VibeGuard/CI commands |

## Planned Change Manifest

| Path | Change |
| --- | --- |
| `src/watch/mod.rs` | Resolve unused timestamped paths and add focused collision/error tests. |

No plan module, schema, watcher mapping, CLI, dependency, documentation, workflow, deletion, or
private-advisory artifact is in scope.

## Risk Analysis

- **Name compatibility:** the no-collision path is byte-for-byte equivalent; focused tests lock it.
- **Silent metadata failure:** `try_exists` errors use existing `PlanError::Io` and stop before write.
- **Counter wrap:** checked arithmetic produces an explicit error.
- **Race boundary:** watch refreshes are serial, which covers the reproduced failure; cross-process
  reservation is not claimed.
- **Trust-model drift:** no scan, selection, ActionPlan content/replay, or delete code changes.

## Verification Plan

```sh
git diff --check
git diff --name-only origin/main...HEAD
cargo test watch::tests
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

## Rollback

Revert the implementation commit. Watch returns to second-level path reuse and may overwrite plans;
there is no migration, schema, dependency, or persistent state to undo.

## Human Gates

- Spec and implementation remain separate PRs.
- Any change to shared ActionPlan I/O, replay, deletion, watcher event selection, or trust policy
  stops this task and requires a new maintainer decision.
- Merge only after current-head CI, zero unresolved review threads and clean merge state under the
  user's standing authorization; never force push.
