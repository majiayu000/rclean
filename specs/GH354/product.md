# Per-Candidate Staleness - Product Spec

GitHub issue: `#354`
Locale: `en-US`
Route: `implement`

## Summary

Compute a cleanup candidate's `staleness_days` from the candidate's own
newest file, not from its parent directory's activity. Today several
caches under one busy parent all read `0d` because an unrelated file in
the parent was touched recently.

## Problem

`staleness_days` is derived from `activity_time`, which
`project_activity` (`src/scan/project.rs:196`) computes as the newest
mtime among the **parent** directory's non-candidate files. One value is
shared by every candidate in that parent.

For a real source project that is the right signal — recent edits to the
source mean its `target/` is probably still wanted. But global caches
whose "project" is just a shared root like `~/Library/Caches` inherit
that root's activity, which is dominated by unrelated OS files.

Measured this session:

```
Yarn cache      2.9 GB   own newest file 2026-07-06 (17 days)   reported 0d
ms-playwright   2.0 GB   own newest file 2026-07-21 ( 2 days)   reported 0d
```

Both sit under `~/Library/Caches`, whose `com.apple.nsservicescache.plist`
is rewritten constantly, so the shared activity time is always ~now.

This contradicts the stated intent of #194: *"Compute 'days since last
meaningful activity' **per candidate**."*

## Impact

Staleness is not display-only. It drives selection:

- `free` proposes stale candidates first
- `free --older-than` / `clean --older-than` filter by activity
- "Biggest wins" ordering and the TUI sort use `staleness_days`

A genuinely stale 2.9 GB cache is therefore de-prioritized, or dropped
by `--older-than 30d`, because of an unrelated sibling.

## Goals

- `staleness_days` reflects the candidate's own newest content.
- Caches under a busy shared parent report their true age.
- No extra filesystem walk: the sizer already visits every file in every
  candidate to sum bytes, so the newest mtime is captured there.
- `risk_score` and the per-project `activity` field keep using the
  existing project-level activity time; only `staleness_days` changes.

## Non-Goals

- Do not change `risk_score` inputs or the risk formula.
- Do not change the `ProjectReport.activity` field semantics.
- Do not change the JSON schema shape (the `staleness_days` field
  already exists; only its computed value becomes more accurate).
- Do not add a new CLI flag or config.

## Behavior Change

`scan --home` under `~/Library/Caches`, before and after:

```
Yarn           cache  2.9 GB  ...  0d   ->  17d
ms-playwright  cache  2.0 GB  ...  0d   ->   2d
```

Candidates already reporting correctly (quiet parents) are unaffected.
`free`'s "stale first" ordering and `--older-than` now act on the true
per-candidate age.

## Acceptance Criteria

1. A candidate whose own newest file is N days old reports
   `staleness_days = N`, even when a sibling under the same parent was
   touched today.
2. A single-file candidate reports staleness from that file's mtime.
3. A candidate the sizer did not walk (blocked / report-only, no size
   computation) falls back to the project activity time rather than
   showing a wrong `0d` or panicking.
4. `risk_score` values are unchanged for the same fixtures.
5. No additional full-tree walk is introduced; the mtime is captured
   during the existing size walk.
6. Deterministic tests use fixed fixture timestamps.
