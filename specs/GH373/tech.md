# GH373 Watch Interval Semantics - Tech Spec

## Linked Artifacts

- GitHub issue: `#373`
- Product spec: `specs/GH373/product.md`
- Tasks: `specs/GH373/tasks.md`
- Route: `plan_first`

```specrail-planned-changes
{
  "issue": 373,
  "complete": true,
  "paths": [
    "CHANGELOG.md",
    "specs/README.md",
    "specs/GH373/product.md",
    "specs/GH373/tech.md",
    "specs/GH373/tasks.md",
    "src/cli.rs",
    "src/watch/mod.rs",
    "tests/cli/diagnostics.rs"
  ],
  "spec_refs": [
    "specs/GH373/product.md",
    "specs/GH373/tech.md",
    "specs/GH373/tasks.md"
  ]
}
```

## Root Cause

`WatchArgs.every` is stored as `String`, so clap accepts any text. Runtime code
then calls `parse_duration`, whose `m` suffix correctly means a month in the
scan-age domain. The repository already has `parse_timeout_duration`, where
`m` means 60 seconds, but watch does not use it.

## Design

Make the interval typed at the CLI boundary:

```rust
#[arg(long, default_value = "60s", value_parser = parse_timeout_duration)]
pub every: Duration,
```

`watch::run_inner` consumes the `Duration` directly. This prevents invalid
intervals from reaching filesystem scanning and removes a runtime parser branch.
No new parser, dependency, or error type is introduced.

## Product-to-Test Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001/B-002 | typed `WatchArgs.every` | parse `5m` and `1h` through `Cli::try_parse_from` |
| B-003 | clap value parser | CLI rejects `watch --every 5d` before scan |
| B-004 | default value remains `60s` | typed default assertion |
| B-005 | `parse_duration` untouched | existing scan-age parser test |
| B-006 | planned-path boundary | scoped diff plus full suite |

## Risks And Mitigations

- **Error wording changes:** invalid values now use clap's value-parser wrapper;
  assert the actionable inner parser message, not clap formatting.
- **Domain coupling:** reuse only the short-duration parser. The age parser and
  all its callers remain untouched.
- **Behavior drift:** parse into `Duration` once, then pass that same value to
  watcher timeout and polling sleep.

## Verification

```sh
cargo test cli::tests::watch_interval_
cargo test --test cli diagnostics::watch_
cargo test parse::tests
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95 cargo build --all-targets --all-features
rustup run 1.95 cargo test
```
