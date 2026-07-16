# Non-Repository Git Discovery Fast Path - Tech Spec

## Linked Artifacts

- GitHub issue: `#259`
- Product spec: `specs/GH259/product.md`
- Tasks: `specs/GH259/tasks.md`
- Route: `write_spec`

## Codebase Context

| Area | Current evidence | Decision |
| --- | --- | --- |
| `src/scan/git_cache.rs:25-33` | `by_dir`, exact `non_repos`, `failed_repos`, global poison flag and timeout | Add one scan-local ancestor marker cache under the same poison discipline. |
| `GitCache::info_for` | Every uncached dir invokes `run_git_rev_parse`, even when siblings share a non-repo parent | Run conservative marker hint before command discovery. |
| `run_git_rev_parse` / `run_git_dirty` | Git is authoritative for root and dirty state | Keep both commands unchanged after a marker/fallback says Git is required. |
| `src/scan/tests.rs` | Existing dirty sibling and non-repo behavior tests | Reuse as unchanged safety regression evidence. |
| `benches/scan_throughput.rs` | Existing 100-small-project and one-huge-candidate Criterion shapes | Use unchanged benchmark for before/after performance evidence. |

## Proposed Internal Design

Add a private marker hint with three states:

```rust
enum MarkerHint {
    Found,
    Absent,
    Fallback,
}
```

`Found` intentionally carries no trusted repo root. It means only that an existing `.git` entry
was seen and the existing Git commands must run. `Absent` is cacheable only after every checked
marker returned `NotFound`. `Fallback` is never cached as absent and always selects the current
command path.

`GitCache` gains a private `marker_by_dir: RwLock<HashMap<PathBuf, bool>>` (or an equivalent
private representation). `true` means a marker exists at that directory or an ancestor;
`false` means no marker was found from that directory upward during this cache lifetime. The
representation must not be exposed through a public method or public `Any`-typed API.

## Lookup Algorithm

1. If Git discovery overrides were captured at `GitCache` construction (`GIT_DIR`,
   `GIT_WORK_TREE`, `GIT_CEILING_DIRECTORIES` or `GIT_DISCOVERY_ACROSS_FILESYSTEM` present),
   return `Fallback`.
2. If the whole cache is poisoned, return `Fallback`.
3. For an uncached queried directory, probe `<dir>/.git` with `symlink_metadata`:
   - any existing entry => `Found`;
   - `NotFound` => continue;
   - any other error => `Fallback`.
4. Only after checking the child marker, consult the exact parent cache. This ordering is the
   B-002 nested-repository guard.
5. Walk parents until a marker, a cached result, or filesystem root is reached. Backfill visited
   directories only for unambiguous `Found`/`Absent` results.
6. `Absent` returns `None` without spawning Git and may populate the existing exact non-repo cache.
7. `Found`/`Fallback` execute the unchanged `run_git_rev_parse`; successful roots continue through
   unchanged `run_git_dirty` and existing `by_dir`/`failed_repos` caching.

Marker probing should be a small private helper whose metadata operation can be injected under
`#[cfg(test)]`, allowing deterministic `NotFound` versus permission/I/O error tests without
platform-dependent chmod fixtures. Git program selection may likewise use a private test-only
constructor so a fake executable can count command invocations without mutating global `PATH`.

## Product-to-Test Mapping

| Invariant | Implementation area | Deterministic verification |
| --- | --- | --- |
| B-001 sibling reuse | marker ancestor cache | fake Git invocation log remains empty for multiple no-marker siblings after first parent result |
| B-002 child marker first | lookup ordering | parent absent cache + sibling `.git` directory still invokes fake/real Git |
| B-003 file and directory | marker probe | temp fixtures for `.git` file and directory both return `Found` |
| B-004 Git-authoritative dirty state | unchanged command path / project report | existing `dirty_git_marks_candidate_caution` and sibling dirty tests |
| B-005 parent repo | upward walk | project below a real temp repo resolves parent root and dirty flag |
| B-006 nested repo | child-first marker | nested initialized repo resolves nested root, not cached parent root |
| B-007 env overrides | constructor-captured override flag | pure/test constructor forces command path despite absent marker |
| B-008 errors and poison | probe injection / lock handling | injected metadata error and poisoned marker lock both invoke fallback path |
| B-009 scan-local state | `GitCache` ownership | new cache instance does not reuse prior marker result; no static/global state |
| B-010 output/perf | unchanged bench + generated fixture | before/after median table, normalized JSON diff, full gates |

## Planned Changes Manifest

| Path | Change |
| --- | --- |
| `src/scan/git_cache.rs` | Add private marker cache, child-first ancestor lookup, conservative fallbacks, and focused unit tests/test seams. |

Existing tests in `src/scan/tests.rs` and the benchmark in `benches/scan_throughput.rs` are
verification inputs and must remain unchanged. No other implementation path is permitted.

## Safety And Concurrency Notes

- Marker results are hints, not classification data. Only Git output creates `GitInfo`.
- All marker cache reads/writes use the existing lock helpers so a poison event flips the shared
  poison flag and future calls recompute through command fallback.
- Never hold a marker-cache lock while spawning or waiting on Git.
- Child `.git` is probed before parent cache reuse to preserve nested repositories.
- Only `NotFound` supports absence; all other I/O kinds are unknown/fallback.
- Environment override state is captured once per `GitCache`, matching its scan-local lifetime and
  avoiding process-environment mutation during parallel tests.

## Performance Verification

Use the same generated 100-sibling fixture and release binary before and after. Capture at least
three warmed default-Git runs per revision on the same machine. Report median real time and require
`before_median / after_median >= 5.0`. Also run the existing Criterion bench and record both shapes.

Capture default JSON for before and after, remove only declared volatile scan metadata, sort keys,
and require an empty diff. Do not compare a `--git-timeout 0` report as the correctness oracle.

## Verification Plan

Focused:

```sh
cargo test scan::git_cache::tests
cargo test scan::tests::git_cache_shares_dirty_flag_across_sibling_projects
cargo test scan::tests::dirty_git_marks_candidate_caution
cargo bench --bench scan_throughput -- --noplot
```

Scope and repository gates:

```sh
git diff --check
git diff --name-only origin/main...HEAD
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

## Rollback

Revert the implementation commit to restore command-per-uncached-dir discovery. The cache has no
persistence, schema, or migration state.

## Human Gates

- Spec and implementation remain separate PRs.
- Merge only after current-head CI, unresolved-thread, merge-state, benchmark, output-diff and
  safety-invariant evidence is green.
- The user has provided standing merge authorization for this optimization run; never force push.
