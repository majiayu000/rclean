# rclean MVP Spec

Status: Draft
Date: 2026-05-05

## Product

`rclean` is a developer workspace artifact cleaner. It scans local project
roots, groups rebuildable artifacts by project, explains safety, and deletes
selected candidates only after an explicit plan.

## Goals

- reclaim disk from rebuildable project artifacts
- make the first scan useful without config
- default to conservative safety behavior
- provide JSON output for scripting
- ship as a Rust-native binary

## Non-Goals

- no background daemon
- no cron/autoclean
- no Docker pruning
- no global package-manager cache cleanup in the default flow
- no GUI
- no plugin system

## Commands

```bash
rclean scan [paths...]
rclean clean [paths...]
rclean explain <path>
rclean rules
```

If no path is provided, `scan` and `clean` default to `.`.

## Core Flags

```bash
--json
--verbose
--depth <n>
--min-size <size>
--older-than <duration>
--category deps,build,cache,test,ide
--rule <rule-id>
--include-ide
--include-caution
--include-blocked
```

Clean-only:

```bash
--all
--dry-run
--permanent
--yes
```

## Rules In MVP

| Rule | Candidate | Category | Safety |
| --- | --- | --- | --- |
| `node.node_modules` | `node_modules` | deps | safe with `package.json` |
| `node.next` | `.next` | build | safe with Node marker |
| `node.turbo` | `.turbo` | cache | safe with Node marker |
| `node.vite` | `.vite` | cache | safe with Node marker |
| `python.venv_dot` | `.venv` | deps | safe with Python and venv markers |
| `python.venv_plain` | `venv` | deps | blocked without venv markers |
| `python.pycache` | `__pycache__` | cache | safe with Python marker |
| `python.pytest` | `.pytest_cache` | cache | safe with Python marker |
| `python.mypy` | `.mypy_cache` | cache | safe with Python marker |
| `python.ruff` | `.ruff_cache` | cache | safe with Python marker |
| `rust.target` | `target` | build | safe with `Cargo.toml`, blocked if shared |
| `go.vendor` | `vendor` | deps | caution with `go.mod` |
| `ios.pods` | `Pods` | deps | safe with `Podfile` |
| `generic.coverage` | `coverage` | test | safe with project marker |

## Safety

`safe` means the candidate matched a deterministic rule and has project marker
evidence. `caution` means the candidate is probably rebuildable but deserves
manual review. `blocked` means the tool must not delete it by default.

Blocked cases:

- symlink candidates
- protected runtime/system paths
- generic directory names without markers
- unvalidated Python `venv`
- shared Cargo target directories

## JSON Contract

`scan --json` emits:

```json
{
  "schemaVersion": 1,
  "toolVersion": "0.1.0",
  "scannedAt": "2026-05-05T00:00:00Z",
  "roots": [],
  "summary": {},
  "projects": []
}
```

Project objects include:

- `path`
- `kind`
- `markers`
- `git`
- `activity`
- `candidates`
- `totalBytes`
- `projectBytes`
- `artifactPercent`

Candidate objects include:

- `path`
- `name`
- `ruleId`
- `category`
- `bytes`
- `safety`
- `reasons`
- `warnings`
- `restoreHint`

## MVP Acceptance

- `cargo test` passes
- root project artifacts are detected
- nested project artifacts are detected
- `scan --json` is valid JSON
- symlink candidates are blocked
- plain Python `venv` without markers is blocked
- dirty git worktrees are caution
- `clean --all --dry-run` prints a plan without deleting
- `clean --all --permanent --yes` deletes only safe selected candidates
