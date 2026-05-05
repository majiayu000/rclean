# rclean

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
- built-in rule listing
- Node, Python, Rust, Go, CocoaPods, and generic coverage rules
- conservative safety classification: `safe`, `caution`, `blocked`
- root-project scanning
- symlink blocking
- dirty git worktree caution
- ActionPlan write/read
- numbered interactive selection
- Java/Gradle, Flutter/Dart, .NET, Ruby, and iOS rules

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
```

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

## Supported Ecosystems

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
