# Per-Candidate Staleness - Tech Spec

GitHub issue: `#354`
Product spec: `specs/GH354/product.md`

## Current Data Flow

1. `scan` maps each candidate to a `project_dir` (its parent).
2. `project_activities(project_dirs, max_depth)` computes one activity
   `SystemTime` per project via `project_activity` — the newest mtime of
   the parent's non-candidate files (`src/scan/project.rs:196`).
3. `build_project_report(..., activity_time)` uses that single value for
   every candidate's `staleness_days` (`:97-100`) and `risk_score`
   (`:92`).

`sizer::summarize` (`src/scan/sizer.rs:114`) already walks every
candidate's whole tree in parallel to sum bytes, reading
`fs::metadata`/`symlink_metadata` at each file.

## Design

Capture the newest file mtime **during the existing size walk** and use
it for `staleness_days`. No second walk.

### 1. `SizeOutcome` carries newest mtime

```rust
struct SizeOutcome {
    bytes: u64,
    newest_mtime: Option<SystemTime>,
    warnings: Vec<ScanWarning>,
}
```

At each of the four metadata reads that already fetch file length, fold
in `metadata.modified().ok()`:

- `dir_size` single-file branch (`:151`)
- `partition_parallel_roots` serial descent (`:224`)
- `dir_size_walk_parallel` parallel `ignore` walk (`:337`)
- `dir_size_walkdir` per-root walkdir (`:446`)

`SizeOutcome::merge` folds `newest_mtime` by max.

For the parallel path, store the newest mtime as an `AtomicU64` of
Unix seconds (0 = none) updated with a compare-exchange max, mirroring
the existing `saturating_atomic_add`. Seconds granularity is enough for
a day-scale staleness metric.

### 2. `SizeSummary` exposes per-candidate activity

```rust
struct SizeSummary {
    candidate_bytes: Vec<u64>,
    candidate_activity: Vec<Option<SystemTime>>,  // parallel to candidate_bytes
    source_bytes: u64,
    warnings: Vec<ScanWarning>,
}
```

### 3. `build_project_report` uses per-candidate activity for staleness

```rust
let candidate_activity = size_summary.candidate_activity[i].unwrap_or(activity_time);
let staleness_days = SystemTime::now()
    .duration_since(candidate_activity)
    .ok()
    .map(|age| age.as_secs() / 86_400);
```

`risk_score` and `ProjectReport.activity` keep using the project-level
`activity_time` unchanged — that signal is intentional for risk, and
only the displayed/consumed staleness was wrong.

The fallback to `activity_time` covers acceptance criterion 3: blocked
candidates get `SizeOutcome::default()` (no walk, `newest_mtime = None`),
so they inherit the project activity rather than a misleading `0d`.

## Why the sizer, not a second walk

The candidate tree is already fully walked for bytes. A separate
`candidate_activity` walk would re-traverse large caches (go-build has
thousands of files) for data the size walk already has in hand. Folding
the max mtime into `SizeOutcome` is O(1) extra work per file and reuses
metadata that is already fetched.

## Files Touched

| File | Change |
| --- | --- |
| `src/scan/sizer.rs` | `SizeOutcome.newest_mtime`; capture at all 4 metadata reads; atomic max for the parallel path; `SizeSummary.candidate_activity`. |
| `src/scan/project.rs` | `staleness_days` from per-candidate activity with fallback to `activity_time`. |
| `src/scan/sizer.rs` tests | Newest-mtime capture across serial, parallel, and single-file paths. |
| `tests/` (scan CLI) | A fixture where a candidate is old but a sibling is fresh, asserting the candidate's true age. |

## Test Plan

Unit (`sizer`):
- A dir whose files have known mtimes reports the newest.
- The parallel path (enough entries to cross
  `PARALLEL_DIRECT_ENTRY_THRESHOLD`) reports the same newest mtime as the
  serial path for the same tree.
- A single-file candidate reports that file's mtime.
- An empty / unwalked candidate reports `None`.

Integration (scan):
- Two candidate dirs under one parent: candidate A last modified 40 days
  ago, a non-candidate sibling touched now. A reports ~40d, not 0d.
  Fails against the pre-change binary.

Determinism: set fixture mtimes explicitly via `std::fs::File::open` +
`set_modified` (stable std, no new dependency). `project/tests.rs`
already uses `OpenOptions` + `FileTimes::set_modified` for the same
purpose.

## Risks

- **Concurrency:** the parallel atomic-max must be correct under
  contention; it follows the proven `saturating_atomic_add` pattern and
  is covered by the serial-vs-parallel equivalence test.
- **Trust model:** none. Sizing bytes, safety classification, and
  ActionPlan handling are unchanged; only the staleness value is
  refined. Safety never depends on staleness.
- **Existing fixtures:** staleness assertions computed from a shared
  parent may shift. Blast radius is small (grep: ~5 assertions); each is
  updated to the now-correct per-candidate value with the reason noted.
