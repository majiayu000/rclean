# ActionPlan JSON format

An ActionPlan is the auditable hand-off between `rclean scan` and
`rclean clean`. The plan is written by:

```bash
rclean scan <paths> --write-plan rclean-plan.json
```

and replayed by:

```bash
rclean clean --plan rclean-plan.json [--dry-run|--yes]
```

Replay uses the plan's persisted `deleteMode`. Passing a conflicting
delete-mode flag at clean time is rejected; regenerate the plan if the
intended mode changed after review.

This document is the contract for that file. Treat any deviation as
a bug.

## Why a separate file format

`rclean scan` is non-destructive. `rclean clean` is destructive. The
ActionPlan is the durable, reviewable artifact in between — the
thing a human (or a code review, or a CI gate) can inspect before
anything gets deleted. The replay path **re-validates every
candidate against the live filesystem** before deletion, so a plan
that becomes stale or that someone tampered with is rejected, not
silently obeyed.

## Schema

The file is UTF-8 JSON written atomically (`tempfile` + rename), so
a partially written plan never appears on disk. The top-level shape:

```json
{
  "schemaVersion": 2,
  "toolVersion": "0.1.4",
  "generatedAt": "2026-05-19T03:21:07.123Z",
  "deleteMode": "trash",
  "roots": ["/Users/me/code"],
  "summary": {
    "projectsScanned": 50,
    "projectsWithCandidates": 47,
    "candidates": 51,
    "safeCandidates": 45,
    "cautionCandidates": 6,
    "blockedCandidates": 0,
    "totalBytes": 478221164544
  },
  "selected": [
    {
      "id": "01KS49DGM3X5AG00000000181R",
      "path": "/Users/me/code/web/node_modules",
      "ruleId": "node.node_modules",
      "bytes": 12884901888,
      "safety": "safe",
      "category": "deps",
      "riskScore": 0.25
    }
  ],
  "projects": [ /* full ProjectReport entries — see schemas below */ ]
}
```

`serde` is configured with `rename_all = "camelCase"` and
`deny_unknown_fields`, so extra keys cause parse errors. Field
ordering in the on-disk JSON is whatever `serde_json::to_string_pretty`
emits (top-level keys appear in struct declaration order).

### Top-level fields

| Field | Type | Notes |
|---|---|---|
| `schemaVersion` | `u32` | Currently `2`. Any other value is rejected on read with a rescan hint. |
| `toolVersion` | `string` | The `CARGO_PKG_VERSION` of `rclean` that wrote the plan. Informational. |
| `generatedAt` | `string` (RFC 3339) | UTC timestamp the plan was written. |
| `deleteMode` | `"trash"` \| `"graveyard"` \| `"permanent"` | What the eventual `clean` will do. Set by the requested clean mode at plan-write time. |
| `roots` | `string[]` | The scan roots, as displayed (typically already canonicalized). `clean --plan` will not delete anything that resolves outside these roots. |
| `summary` | object | Counts and total bytes for the **selected** subset, not the whole scan (with `projectsScanned` / `projectsWithCandidates` preserved from the underlying scan). See below. |
| `selected` | `PlanCandidate[]` | The list of paths `clean --plan` will attempt to delete, in scan-emit order. Built from non-sudo `Safety::Safe` candidates (and non-sudo `Safety::Caution` if `--include-caution` was passed to the scan). |
| `projects` | `ProjectReport[]` | Full per-project detail, identical to the `--json` scan report's `projects` array. Provides reviewer context — it does *not* drive deletion. |

### `selected[i]` — `PlanCandidate`

| Field | Type | Notes |
|---|---|---|
| `id` | `string` | Stable 26-character candidate id generated when the plan is written. Graveyard mode records it for later restore/audit trails. |
| `path` | `string` | Absolute path to the candidate directory. |
| `ruleId` | `string` | The rule id (e.g. `rust.target`, `node.node_modules`) recorded at scan time. Re-validated at clean time. |
| `bytes` | `u64` | Candidate byte size at scan time. Informational — `clean --plan` does not enforce it. |
| `safety` | `"safe"` \| `"caution"` \| `"blocked"` \| `"report-only"` \| `"unknown"` | Safety tier at scan time. In a plan written by the standard flow this is always `"safe"` or `"caution"` — `"blocked"`, `"report-only"`, and `"unknown"` are filtered out by `collect_selected`. Re-validated at clean time. |
| `requiresSudo` | `bool` | Defaults to `false` when absent in older schema-version-2 plans. When `true`, replay refuses cleanup because `rclean` does not run `sudo`. |
| `category` | `"deps"` \| `"build"` \| `"cache"` \| `"test"` | Candidate category captured from the scan report. |
| `riskScore` | `f32` | Advisory risk score captured from the scan report. It does not gate deletion. |

### `summary`

| Field | Type | Notes |
|---|---|---|
| `projectsScanned` | `usize` | From the underlying scan. Not "how many projects the plan touches". |
| `projectsWithCandidates` | `usize` | From the underlying scan. |
| `candidates` | `usize` | Count of `selected[]`. |
| `safeCandidates` | `usize` | Count of `selected[]` with `safety == "safe"`. |
| `cautionCandidates` | `usize` | Count of `selected[]` with `safety == "caution"`. |
| `blockedCandidates` | `usize` | Should be `0` in any plan written by the standard flow. |
| `totalBytes` | `u64` | Sum of `selected[].bytes`. |

## Replay semantics

`rclean clean --plan rclean-plan.json` runs in two phases.

### 1. Parse + classifier revalidation (`selected_from_action_plan`)

For each entry in `selected[]`:

- Reject if the path is inside a runtime/system path (`.cargo`,
  `.rustup`, `Library`, etc.) — the SafetyMode allowlist.
- Reject if `requiresSudo == true`; these entries require manual
  administrator cleanup and `rclean` will not run `sudo`.
- Look up the candidate against the **current** built-in rule set.
- Reject if no built-in rule recognizes the path now — the plan is
  stale or tampered with.
- Reject if the rule classifies it as `Blocked` or `Unknown` now —
  even if the plan claimed `safe`.

This guards against:

- A plan generated on an older `rclean` whose rules have since
  tightened. The newer classifier always wins.
- A plan that was hand-edited to upgrade a `blocked` path to `safe`.
- A plan that names a path that has nothing to do with rclean's
  rules.

### 2. Filesystem revalidation (`revalidate_selected`)

For each entry in `selected[]`:

- Canonicalize every plan root; reject the whole plan if no root
  canonicalizes.
- `symlink_metadata(path)`: reject if the path is now a symlink.
- Reject if the path is no longer a directory.
- Reject if the canonical path does not start with any plan root —
  the path was moved out of the scan tree (or was always outside).

### 3. Broad-root guard

Even after the two revalidation steps pass, `clean::check_broad_roots`
refuses to delete inside `/`, `$HOME`, `/etc`, `/usr` unless
`--allow-broad-root` is passed.

### 4. Deletion

Only after all of the above pass does `clean::delete_selected` move
each path according to the plan's `deleteMode`: Trash, permanent
deletion, or the graveyard. Clean-time delete-mode flags are allowed
only when they match the plan; they cannot override the reviewed
mode.

A failure in any of the validation steps aborts the entire clean —
nothing is deleted partially.

## What the plan does *not* guarantee

- **Size accuracy.** The `bytes` field is a snapshot. The directory
  may have grown or shrunk before clean time. `clean --plan` does
  not enforce it.
- **Plan-time safety.** The `safety` field is informational. The
  classifier and filesystem revalidation steps above are what
  actually gate deletion.
- **Cross-version compatibility forever.** `schemaVersion` is the
  versioning hook. Current plans use `2`. Future schemas bump this
  field and may not be replayable by older binaries.

## Versioning policy

- Current builds write `schemaVersion: 2`. Schema 1 plans are rejected
  with a message telling the user to re-run `rclean scan --write-plan`.
- New additive fields land with explicit `serde(default)` to keep
  forward-compat within the same `schemaVersion`.
- Renames or removals require a `schemaVersion` bump.
- The schema number is independent of the tool version; a later release
  that doesn't change the file format keeps `schemaVersion: 2`.

## Integration patterns

### Code-review the plan

```bash
rclean scan ~/code --write-plan rclean-plan.json
git add rclean-plan.json
git commit -m "review: rclean plan for monorepo cleanup"
# open PR, review the .selected[] array, approve, then run:
rclean clean --plan rclean-plan.json --yes
```

The plan is a portable, diffable artifact. Reviewers see exactly
which absolute paths will go and which rule id justified each one.

### CI guard

```bash
# Build a plan in CI, fail if it includes a path you've banned.
rclean scan . --write-plan rclean-plan.json
jq -e '.selected[] | select(.path | test("vendored/"))' rclean-plan.json \
  && { echo "forbidden path made it into the plan"; exit 1; } \
  || true
```

### Replay across machines

The plan is content-addressed by absolute path, so it only replays
on the same machine. There is no plan format intended for
cross-machine replay — the trust model relies on canonicalized roots
existing locally.

## Related

- [`docs/explain-mode.md`](explain-mode.md) — per-path inspector;
  the same classifier revalidation step lives inside the explain path.
- [`docs/architecture.md`](architecture.md) — where `plan.rs` sits in
  the scan → clean pipeline.
- [`SECURITY.md`](../SECURITY.md) — threat model behind the
  revalidation rules.
