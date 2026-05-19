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
- **global toolchain caches**: Cargo registry, npm `_cacache`,
  yarn cache, pip cache, Gradle caches, Maven local repo, Xcode
  `DerivedData`, iOS Simulators (via `scan --home`)
- conservative safety classification: `safe`, `caution`, `blocked`
- root-project scanning
- symlink blocking
- dirty git worktree caution
- ActionPlan write/read
- numbered interactive selection

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

`--home` expands to `~/.cargo`, `~/.gradle`, `~/.m2`, `~/.npm`,
`~/.pnpm-store`, plus `~/Library/Caches` and `~/Library/Developer`
on macOS or `~/.cache` on Linux. Paths that don't exist are
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
| `node.npm_cacache` | `~/.npm/_cacache` | safe | next `npm install` |
| `node.yarn_cache` | `~/Library/Caches/Yarn` (macOS) | safe | next `yarn install` |
| `pip.cache` | `~/Library/Caches/pip` (macOS) / `~/.cache/pip` (Linux) | safe | next `pip install` |
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
node.npm_cacache           applicable ~/.npm
pip.cache                  applicable ~/Library/Caches
node.yarn_cache            applicable ~/Library/Caches
xcode.derived_data         applicable ~/Library/Developer/Xcode
xcode.simulators           applicable ~/Library/Developer
gradle.caches              skipped    no Gradle install detected
maven.local_repo           skipped    no Maven install detected

7 of 9 rules applicable on this machine.
```

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

## Development

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
cargo run --bin rclean -- scan . --json
```
