# rclean

[![CI](https://github.com/majiayu000/rclean/actions/workflows/ci.yml/badge.svg)](https://github.com/majiayu000/rclean/actions/workflows/ci.yml)
[![Audit](https://github.com/majiayu000/rclean/actions/workflows/audit.yml/badge.svg)](https://github.com/majiayu000/rclean/actions/workflows/audit.yml)

Rust-native CLI for finding and cleaning rebuildable developer artifacts.

`rclean` is not a general disk cleaner. It targets project-local artifacts
that can be recreated from source, lockfiles, package managers, or build tools:
`node_modules`, `.next`, `.venv`, `target`, Python caches, Turborepo/Vite caches,
and similar directories.

The trust model is the product: scan first, explain every candidate, write an
ActionPlan when you want a reviewable cleanup, and never select blocked paths.

Real local benchmark:

```text
50 projects scanned
51 candidates
445 GB reclaimable above 100 MB
largest candidate: harness-workflow-runtime-phase2/target at 101 GB
```

![rclean scan demo](docs/assets/rclean-demo.svg)

## Current Status

This is a from-scratch Rust CLI. It already supports:

- `scan` with human table output
- "Biggest wins" scan summary with project artifact percentage
- `scan --json`
- `clean --dry-run`
- `clean --all --permanent --yes`
- `explain <path>`
- built-in rule listing (`rules`)
- per-machine diagnostic (`doctor`)
- Node, Python, Rust, Go, CocoaPods, and generic coverage rules
- Java/Gradle, Flutter/Dart, .NET, Ruby, and iOS rules
- **global toolchain caches**: Cargo registry, Go module/build
  cache, npm `_cacache`, pnpm store, yarn cache, pip cache, Bun install
  cache, Deno cache, Gradle caches, Maven local repo, Xcode
  `DerivedData`, iOS Simulators (via `scan --home`)
- conservative safety classification: `safe`, `caution`, `blocked`
- root-project scanning
- symlink blocking
- dirty git worktree caution
- ActionPlan write/read
- numbered interactive selection
- `agent doctor codex` for local Codex process, disk, power, and update diagnostics
- `agent optimize codex --disable-auto-update` as a dry-run-first one-shot setting helper

## Why rclean

Existing tools already clean `node_modules`, `target`, and other artifacts.
`rclean` focuses on the part that makes people hesitate before deleting:

- clear safety states: `safe`, `caution`, `blocked`
- immediate top cleanup wins before the detailed table
- reviewable ActionPlan JSON
- symlink and root-boundary revalidation before plan-based cleanup
- dirty git worktrees marked as caution
- package name `rclean-cli`, installed command `rclean`

## Install

From this checkout during development:

```bash
cargo install --path .
```

After public release, the intended install path is:

```bash
cargo install rclean-cli
```

The Cargo package is `rclean-cli`; the installed command is `rclean`.

## Usage

```bash
cargo run --bin rclean -- scan ~/code
cargo run --bin rclean -- scan ~/code --json
cargo run --bin rclean -- clean ~/code --all --dry-run
cargo run --bin rclean -- clean ~/code --all --permanent --yes
cargo run --bin rclean -- explain ~/code/app/target
cargo run --bin rclean -- rules
cargo run --bin rclean -- doctor
cargo run --bin rclean -- scan --home
cargo run --bin rclean -- agent doctor codex
cargo run --bin rclean -- agent optimize codex --disable-auto-update
```

### Whole-machine cleanup

`rclean scan --home` is the convenience entry point for cleaning
**every cache a developer toolchain leaves under `$HOME`** without
listing each path:

```bash
rclean doctor                                # see which global rules apply
rclean scan --home --min-size 100mb          # report candidates
rclean scan --home --write-plan plan.json    # auditable plan
rclean clean --plan plan.json --dry-run      # preview
rclean clean --plan plan.json --yes          # execute (defaults to Trash)
```

`--home` expands to `~/.cargo`, `~/go`, `~/.gradle`, `~/.m2`,
`~/.npm`, `~/.pnpm-store`, plus `~/Library/Caches`,
`~/Library/pnpm`, and `~/Library/Developer` on macOS or `~/.cache`
and `~/.local/share/pnpm` on Linux. Existing
`GOPATH` entries are included too. Paths that don't exist are
filtered out silently. See the
[Global Toolchain Caches](#global-toolchain-caches) table below
for the full rule list.

After installation:

```bash
rclean scan ~/code
rclean clean ~/code --all --dry-run
```

Write and review an ActionPlan:

```bash
rclean scan ~/code --write-plan rclean-plan.json
rclean clean --plan rclean-plan.json --dry-run
```

## Safety Model

- `scan` never deletes files.
- blocked candidates are never selected by `clean --all`.
- symlink candidates are blocked.
- generic directories like `build`, `dist`, `out`, `target`, and `vendor` require
  project marker evidence.
- Python `venv` must contain virtualenv markers.
- dirty git worktrees downgrade otherwise safe candidates to `caution`.
- `--all` selects only `safe` candidates unless `--include-caution` is passed.
- default clean mode moves to Trash when available.
- `--permanent` is required for permanent deletion.

See [`SECURITY.md`](SECURITY.md) for the threat model, in-scope issues,
and how to report a vulnerability privately.

## Supported Ecosystems

### Project-level artifacts

These rules fire inside a project directory (require a marker like
`Cargo.toml`, `package.json`, etc.):

| Ecosystem | Examples |
| --- | --- |
| Node/JS | `node_modules`, `.next`, `.turbo`, `.vite`, `.parcel-cache`, `dist`, `build`, `out` |
| Python | `.venv`, `venv`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.tox` |
| Rust | `target` |
| Go | `vendor` |
| iOS | `Pods` |
| Java/Gradle | `target`, `build`, `.gradle` |
| Flutter/Dart | `build`, `.dart_tool` |
| .NET | `bin`, `obj` |
| Ruby | `.bundle`, `vendor/bundle` |

### Global toolchain caches

These rules fire on caches the toolchains maintain *outside*
individual projects, under `$HOME`. Use `rclean scan --home` to
let rclean find every applicable cache automatically:

| Rule id | Path | Safety | Restore |
| --- | --- | --- | --- |
| `cargo.registry_cache` | `~/.cargo/registry/cache` | safe | next `cargo build` |
| `cargo.git_db` | `~/.cargo/git/db` | safe | next `cargo build` |
| `go.module_download_cache` | `~/go/pkg/mod/cache/download` / `$GOPATH/pkg/mod/cache/download` | safe | next `go build` / `go test` |
| `go.build_cache` | `~/Library/Caches/go-build` (macOS) / `~/.cache/go-build` (Linux) | safe | next `go build` / `go test` |
| `node.npm_cacache` | `~/.npm/_cacache` | safe | next `npm install` |
| `node.pnpm_store` | `~/.pnpm-store/vN` / `~/Library/pnpm/store` (macOS) / `~/.local/share/pnpm/store` (Linux) | safe | next `pnpm install` |
| `node.yarn_cache` | `~/Library/Caches/Yarn` (macOS) | safe | next `yarn install` |
| `pip.cache` | `~/Library/Caches/pip` (macOS) / `~/.cache/pip` (Linux) | safe | next `pip install` |
| `js.bun_install_cache` | `~/.bun/install/cache` (sub-path only — `~/.bun` itself is never selected) | caution | `bun pm cache rm` |
| `js.deno_cache` | `~/Library/Caches/deno` (macOS) / `~/.cache/deno` (Linux) | caution | `deno cache --reload` |
| `gradle.caches` | `~/.gradle/caches` | caution | next Gradle build |
| `maven.local_repo` | `~/.m2/repository` | caution | next `mvn install` |
| `xcode.derived_data` | `~/Library/Developer/Xcode/DerivedData` | safe | next Xcode build |
| `xcode.simulators` | `~/Library/Developer/CoreSimulator` | caution | next iOS app run |

Run `rclean doctor` to see which of these apply on your machine
right now:

```
$ rclean doctor

Rule                       Status     Anchor / Reason
----------------------------------------------------------------------------
cargo.registry_cache       applicable ~/.cargo/registry
cargo.git_db               applicable ~/.cargo/git
go.module_download_cache   applicable ~/go/pkg/mod/cache
go.build_cache             applicable ~/Library/Caches/go-build
node.npm_cacache           applicable ~/.npm
node.pnpm_store            skipped    no pnpm store detected
pip.cache                  applicable ~/Library/Caches
node.yarn_cache            applicable ~/Library/Caches
xcode.derived_data         applicable ~/Library/Developer/Xcode
xcode.simulators           applicable ~/Library/Developer
gradle.caches              skipped    no Gradle install detected
maven.local_repo           skipped    no Maven install detected

9 of 14 rules applicable on this machine.
```

User records are not cleanup candidates. The following paths are
treated as protected user data and refused at scan, plan replay, and
delete time — even if a custom rule or tampered ActionPlan points at
them:

- `~/.codex/sessions`, `~/.codex/memories`
- `~/.claude/projects`, `~/.claude/sessions`, `~/.claude/history.jsonl`,
  `~/.claude/shell-snapshots`, `~/.claude/file-history`,
  `~/.claude/todos`

## Custom Rules (`.rclean.toml`)

Drop a `.rclean.toml` at the scan root to teach `rclean` about
project-specific artifact directories that aren't in the built-in
catalog. Each rule is a `[[rule]]` table:

```toml
[[rule]]
id = "myproj-prebuilt"
name_glob = "prebuilt"
parent_markers = ["pyproject.toml", "build.config.json"]
category = "build"
safety = "caution"
why = "regenerated by build.sh; remove to force a clean rebuild"
restore_hint = "run ./build.sh"

[[rule]]
id = "myproj-cache"
name_glob = ".myproj-cache"
parent_markers = [".myproj"]
category = "cache"
safety = "safe"
why = "myproj evaluation cache, recreated on next run"
```

Fields:

| Field | Required | Notes |
| --- | --- | --- |
| `id` | yes | Unique within the file. Duplicate ids: first wins, the rest are skipped with a warning. |
| `name_glob` | yes | `globset` glob matched against the candidate directory name (e.g. `prebuilt`, `*.cache`). |
| `parent_markers` | no | Files or directories that must exist in the candidate's parent to enable the rule. Any one marker is enough (OR). |
| `category` | yes | One of `deps`, `build`, `cache`, `test`. |
| `safety` | no | `safe` (default) or `caution`. `blocked` is rejected — only built-in rules may produce blocked. `caution` requires at least one `parent_markers` entry, so a bare-name caution rule cannot fire under arbitrary directories. |
| `why` | no | One-line reason shown in the report. Defaults to `matches user rule '<id>'`. |
| `restore_hint` | no | Short hint for `rclean explain` output. |

Invalid rules emit a `warning:` line on stderr and are dropped; the
scan continues with the remaining rules. A missing `.rclean.toml` is
the normal case and produces no warning.

User rules layer *after* built-in rules: if a directory already
matches a built-in rule (e.g. `node_modules`), the user rule never
fires for that directory.

## Examples

Scan a workspace:

```bash
rclean scan ~/code --min-size 100mb
```

Only find old dependency/build artifacts:

```bash
rclean scan ~/code --older-than 6m --category deps,build
```

Machine-readable report:

```bash
rclean scan ~/code --json > rclean-report.json
```

Preview a bulk clean:

```bash
rclean clean ~/code --all --dry-run
```

Permanent clean after reviewing the dry run:

```bash
rclean clean ~/code --all --permanent --yes
```

## Filtering

`rclean` supports several flags for narrowing what `scan` and `clean` consider.
All filters apply *after* classification, so blocked paths are still suppressed
from the bulk selection regardless of these settings.

| Flag | Default | Effect |
| --- | --- | --- |
| `--depth <N>` | `6` | Max directory levels traversed from each root. |
| `--min-size <SIZE>` | `1mb` | Drop candidates smaller than `SIZE` (e.g. `0`, `100mb`, `1g`). Blocked candidates are never dropped by size. |
| `--older-than <DUR>` | none | Keep only projects whose newest activity is older than `DUR` (e.g. `30d`, `6m`, `1y`). |
| `--category <LIST>` | all | Comma-separated subset of `deps,build,cache,test`. |
| `--rule <LIST>` | all | Comma-separated rule ids (see `rclean rules`). |
| `--include-caution` | off | Include caution candidates in `clean --all`. |
| `--include-blocked` | off | Show blocked candidates in the report. They are still never selected by `--all`. |
| `--ignore <GLOB>` | none | Repeatable. Drops candidates matching a `.gitignore`-style glob. |
| `--allow-broad-root` | off | `clean` only. Allow a scan root that resolves to a broad system or user path (e.g. `/`, `$HOME`, `/etc`, `/usr`). |

## `.rcleanignore`

Place an `.rcleanignore` file at the root of any scan target to permanently
exclude candidate paths. The syntax is the same as `.gitignore`, including
negation with `!`:

```gitignore
# Keep a vendored target tree we deliberately ship
sealed-vendor/target

# Skip an entire workspace
legacy-monorepo/

# But re-include one project inside it
!legacy-monorepo/important-app/node_modules
```

`--ignore <GLOB>` layers on top of `.rcleanignore` and is repeatable, so
ad-hoc exclusions don't need a file:

```bash
rclean scan ~/code --ignore "**/playground/**" --ignore "tmp-*"
```

If both an `.rcleanignore` entry and a `--ignore` glob match the same path,
the path is excluded.

## Reports and Plans

`rclean scan` can emit a machine-readable JSON report or an auditable action
plan:

```bash
# JSON for tooling/CI
rclean scan ~/code --json > rclean-report.json

# Action plan for human review and replayable cleanup
rclean scan ~/code --write-plan rclean-plan.json
rclean clean --plan rclean-plan.json --dry-run
rclean clean --plan rclean-plan.json --yes
```

The action plan is the trust boundary: `clean --plan` re-validates every
path against the live filesystem before deleting, refuses to follow new
symlinks, and rejects plans whose roots have changed shape since the
scan.

## Development

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
cargo run --bin rclean -- scan . --json
```
