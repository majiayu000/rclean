# rclean Complete Spec

Status: Active
Date: 2026-05-06
Package: `rclean-cli`
Binary: `rclean`

## 1. Product Definition

`rclean` is a Rust-native CLI for finding and cleaning rebuildable developer
artifacts across local project workspaces.

It is not a general disk cleaner. It targets project-local directories that can
be recreated from source, lockfiles, package managers, or build tools.

The core promise:

> Show how much developer artifact bloat exists, explain why each candidate is
> safe or risky, and delete only from an explicit plan.

## 2. Goals

- Scan one or more workspace roots and group artifacts by project.
- Explain every cleanup candidate with rule evidence and restore hints.
- Classify candidates as `safe`, `caution`, `blocked`, or `unknown`.
- Support human tables and stable JSON.
- Support ActionPlan write/read flows for reviewable cleanup.
- Provide safer defaults than raw `rm -rf`.
- Ship with tests, CI, release docs, and benchmark evidence.

## 3. Non-Goals

- No cron/autoclean.
- No background daemon.
- No global package-manager cache cleanup in the default flow.
- No Docker pruning in this phase.
- No GUI in this phase.
- No plugin system in this phase.

## 4. Commands

```bash
rclean scan [paths...]
rclean clean [paths...]
rclean explain <path>
rclean rules
```

If no path is passed to `scan` or `clean`, the current directory is used.

## 5. Flags

Common scan flags:

```bash
--json
--verbose
--depth <n>
--min-size <size>
--older-than <duration>
--category <deps,build,cache,test,ide>
--rule <rule-id>
--include-ide
--include-caution
--include-blocked
```

Plan flags:

```bash
--write-plan <path>
--plan <path>
```

Clean flags:

```bash
--all
--dry-run
--permanent
--yes
```

## 6. ActionPlan Contract

An ActionPlan is JSON with schema version `1`.

Required fields:

- `schemaVersion`
- `toolVersion`
- `generatedAt`
- `deleteMode`
- `roots`
- `summary`
- `selected`
- `projects`

Plan semantics:

- `scan --write-plan plan.json` writes all report candidates to an ActionPlan.
- `clean --plan plan.json --dry-run` prints selected candidates without deleting.
- `clean --plan plan.json` revalidates every selected candidate before deletion.
- stale plan paths must be rejected if they no longer exist, become symlinks, or
  resolve outside the original roots.

Default selected set:

- `safe` candidates are selected.
- `caution` candidates are included in the plan but not selected unless
  `--include-caution` is used.
- `blocked` candidates are included only when `--include-blocked` is used and are
  never selected automatically.

## 7. Safety Model

`safe`:

- deterministic rule matched
- required project marker exists
- path resolves under requested root
- path is not a symlink
- path is not protected runtime/system location
- artifact is not known shared state

`caution`:

- git worktree is dirty
- artifact is rebuildable but may contain intentionally cached or checked-in data
- rule is broad or generic

`blocked`:

- symlink candidate
- path outside requested root after canonicalization
- protected runtime/system path
- generic directory without project marker
- unvalidated Python `venv`
- shared Cargo target

`clean --all` only selects `safe` candidates unless `--include-caution` is set.

## 8. Rule Catalog

### Node

| Rule | Candidate | Category | Safety |
| --- | --- | --- | --- |
| `node.node_modules` | `node_modules` | deps | safe |
| `node.next` | `.next` | build | safe |
| `node.turbo` | `.turbo` | cache | safe |
| `node.vite` | `.vite` | cache | safe |
| `node.parcel` | `.parcel-cache` | cache | safe |
| `node.dist` | `dist` | build | caution |
| `node.build` | `build` | build | caution |
| `node.out` | `out` | build | caution |

### Python

| Rule | Candidate | Category | Safety |
| --- | --- | --- | --- |
| `python.venv_dot` | `.venv` | deps | safe if validated |
| `python.venv_plain` | `venv` | deps | safe if validated, blocked otherwise |
| `python.pycache` | `__pycache__` | cache | safe |
| `python.pytest` | `.pytest_cache` | cache | safe |
| `python.mypy` | `.mypy_cache` | cache | safe |
| `python.ruff` | `.ruff_cache` | cache | safe |
| `python.tox` | `.tox` | cache | caution |

### Rust

| Rule | Candidate | Category | Safety |
| --- | --- | --- | --- |
| `rust.target` | `target` | build | safe unless shared |

### Java And Gradle

| Rule | Candidate | Category | Safety |
| --- | --- | --- | --- |
| `java.maven_target` | `target` | build | safe |
| `java.gradle_build` | `build` | build | safe |
| `java.gradle_cache_local` | `.gradle` | cache | caution |

### Flutter/Dart

| Rule | Candidate | Category | Safety |
| --- | --- | --- | --- |
| `dart.build` | `build` | build | safe |
| `dart.tool` | `.dart_tool` | cache | safe |

### .NET

| Rule | Candidate | Category | Safety |
| --- | --- | --- | --- |
| `dotnet.bin` | `bin` | build | safe |
| `dotnet.obj` | `obj` | build | safe |

### Ruby

| Rule | Candidate | Category | Safety |
| --- | --- | --- | --- |
| `ruby.bundle` | `.bundle` | cache | caution |
| `ruby.vendor_bundle` | `vendor/bundle` | deps | caution |

### iOS

| Rule | Candidate | Category | Safety |
| --- | --- | --- | --- |
| `ios.pods` | `Pods` | deps | safe |

### Generic

| Rule | Candidate | Category | Safety |
| --- | --- | --- | --- |
| `generic.coverage` | `coverage` | test | safe with marker |

## 9. Interactive Selection

`rclean clean` without `--all` should not require the user to trust an opaque
bulk selection.

Required behavior:

- print a numbered list grouped by project
- include safety, category, size, and reason
- accept comma-separated selections such as `1,3,5`
- accept ranges such as `2-6`
- accept `a` for all safe candidates
- accept empty input for none
- never include blocked candidates

## 10. JSON Output

`scan --json` emits schema version `1` with:

- roots
- summary
- projects
- project markers
- git status
- activity
- candidates

Candidate fields:

- `path`
- `name`
- `ruleId`
- `category`
- `bytes`
- `safety`
- `reasons`
- `warnings`
- `restoreHint`

## 11. Benchmark Evidence

The repository must include a benchmark report under `docs/reports/` from scanning
the local workspace root.

Minimum report fields:

- command
- date
- duration
- projects scanned
- candidates
- reclaimable bytes
- largest candidates
- false-positive notes

## 12. Release Requirements

Before public release:

- GitHub Actions CI for fmt, clippy, test, and release build
- README install instructions
- release checklist
- crates.io package naming note: package is `rclean-cli`, binary is `rclean`
- GitHub Release binary instructions
- Homebrew formula plan

## 13. Acceptance Criteria

This phase is complete when:

- current MVP baseline is committed
- complete spec exists
- ActionPlan write/read dry-run works
- plan clean revalidates selected paths
- interactive selection accepts lists and ranges
- Java/Gradle, Flutter/Dart, .NET, Ruby, and extra Node/Python rules exist
- tests cover ActionPlan, selection parser, and new rules
- benchmark report exists
- release docs and CI workflow exist
- `cargo fmt -- --check` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo test` passes
- `cargo build --release` passes
