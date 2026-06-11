# `rclean explain`

`explain` is the one-shot inspector. Given a path, it tells you
which built-in rule (if any) would classify it, what safety tier it
falls into, why, and what `clean` would do with it. It is the
primary integration surface for CI gates, pre-delete sanity checks,
and AI-agent loops that need a stable verdict on a single path
without running a full scan.

## Usage

```bash
rclean explain [--activity-depth <N>] <PATH>
```

`<PATH>` may be relative or absolute. `explain` does not require the
path to exist as a directory; it inspects the symlink shape, the
file name, and the parent directory's markers. It never writes
anything and never deletes anything.

`--activity-depth` controls how deeply `explain` walks the parent
project when computing the activity portion of `Risk`. It defaults to
`6`, matching the default scan traversal depth.

## Sample output

A classified candidate inside a real project:

```
$ rclean explain ./target
Path: ./target
Safety: safe
Rule: rust.target
Category: build
Reasons:
  - Cargo.toml marker found
Restore: Run cargo build or cargo test
Risk: 0.25
```

An unrelated path that no built-in rule matches:

```
$ rclean explain /etc/passwd
Path: /etc/passwd
Safety: unknown
Reasons:
  - no built-in rule matched this path
No built-in cleanup rule matched this path.
```

A symlinked candidate (always Blocked):

```
$ rclean explain ./node_modules     # where ./node_modules -> /elsewhere
Path: ./node_modules
Safety: blocked
Rule: node.node_modules
Category: deps
Reasons:
  - npm/pnpm/yarn install target
Warnings:
  - candidate is a symlink
Restore: Run npm install / pnpm install / yarn install
```

## Exit codes

`explain` is designed to be safe to script against. Every safety
state maps to a stable exit code:

| Exit code | Meaning | Trigger |
|---:|---|---|
| `0` | Classified `safe` or `caution` | A built-in rule matched and the path is selectable by `clean`. |
| `3` | `unknown` | No built-in rule matched. Treat as "out of scope" — `clean` will never touch this path. |
| `4` | `blocked` | A built-in rule matched but a safety check (symlink, runtime path, scan-root violation) demoted it. `clean --all` will never select this path. |
| `1` | Argument or I/O error | Bad path, canonicalize failure, etc. — error message goes to stderr. |

Exit codes 0/3/4 cleanly separate the three states a CI script
typically cares about: "go ahead and clean", "ignore this path",
"hard refuse to clean".

## Fields

| Field | When present | Source |
|---|---|---|
| `Path` | Always | The argument, as passed in (not canonicalized). |
| `Safety` | Always | `safe` / `caution` / `blocked` / `unknown`. |
| `Rule` | When a built-in rule matched | The rule id (e.g. `rust.target`, `node.node_modules`). Matches the values from `rclean rules`. |
| `Category` | When a rule matched | `deps`, `build`, `cache`, or `test`. |
| `Reasons` | Always (one or more lines) | The rule's justification (e.g. `Cargo.toml marker found`). For `unknown`, a single "no built-in rule matched" line. |
| `Warnings` | When a safety check downgraded the path | `candidate is a symlink`, `candidate is inside a protected runtime or system path`, `project has uncommitted git changes`, etc. |
| `Restore` | When a rule matched | One-line recovery hint (e.g. `Run cargo build`). Matches the rule's catalog entry. |
| `Risk` | When a rule matched | Advisory composite score in `[0.0, 0.85]`. Combines dirty-git (0.40), recent activity (0.25), no-lockfile (0.20). `--activity-depth` controls the recent-activity traversal for `explain`. See [`docs/specs/v0.1.x-roadmap.md`](specs/v0.1.x-roadmap.md) §4.6. |

## Risk vs. Safety

`Safety` is the operational gate — it controls whether `clean` is
willing to touch the path. `Risk` is an advisory analytical signal,
emitted alongside `Safety` so downstream consumers (TUI coloring,
agent plan scoring) can sort or weight candidates without
re-deriving the same data.

A `safe` candidate can still have a non-zero risk score (e.g. a
recently modified project with no lockfile). A `blocked` candidate
still gets a risk score reported, but `clean --all` will never
select it regardless of the score.

The two signals will never collapse into one. Do not gate `clean`
on `Risk` — gate on `Safety` only.

## Integration patterns

### Pre-delete sanity check in a shell script

```bash
# Refuse to rm the path if rclean considers it blocked or unknown.
rclean explain "$path" >/dev/null
case $? in
  0) rm -rf "$path" ;;          # safe / caution
  3) echo "skip: $path (out of scope)" ;;
  4) echo "refuse: $path (blocked)"; exit 1 ;;
  *) echo "error: $path"; exit 1 ;;
esac
```

### CI gate that fails the build if a forbidden path is selectable

```bash
# A repo invariant: vendored/ must never be cleanable by rclean.
rclean explain ./vendored
test $? -eq 4 || { echo "vendored is selectable — fix rules"; exit 1; }
```

### Agent loop that delegates the safety decision

An AI agent that wants to clean an artifact directory should call
`rclean explain` first, parse the safety tier, and only proceed
when the exit code is `0` and the `Safety:` line is `safe`
(`caution` requires `--include-caution` and an explicit confirmation
step). Treating exit code `4` as "do nothing and surface to the
user" is the recommended pattern — it matches `clean --all`'s
behavior exactly.

## What `explain` does *not* do

- It does not compute reclaimable bytes — that requires a full scan.
- It does not consult `.rclean.toml` user rules (only built-in
  rules). User rules are only evaluated during a real `scan` so
  their `parent_markers` resolve correctly inside the scan tree.
- It does not honor `.rcleanignore` — that is a `scan`-time filter,
  not a per-path classifier.
- It does not warn about dirty git (the demotion to `caution` only
  happens inside a real scan, where the project context is known).
  The `Risk` axis does still report the dirty-git contribution.

If you need any of those signals, run `scan` with a narrow root
(`rclean scan <project>`) instead.

## Related

- [`docs/architecture.md`](architecture.md) — where `explain`
  lives in the pipeline.
- [`README.md`](../README.md) — usage examples and supported
  ecosystems.
- [`docs/specs/v0.1.x-roadmap.md`](specs/v0.1.x-roadmap.md) — risk
  score formula and roadmap.
