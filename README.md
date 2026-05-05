# rclean

Rust-native CLI for finding and cleaning rebuildable developer artifacts.

`rclean` is not a general disk cleaner. It targets project-local artifacts
that can be recreated from source, lockfiles, package managers, or build tools:
`node_modules`, `.next`, `.venv`, `target`, Python caches, Turborepo/Vite caches,
and similar directories.

## Current Status

This is a from-scratch MVP. It already supports:

- `scan` with human table output
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
cargo run -- scan . --json
```
