# GH159 technical spec: Docker cleanup safety gates

## Current Constraint

`rclean` currently models cleanup candidates as filesystem paths. Scan output,
ActionPlan JSON, deletion validation, trash/graveyard behavior, and audit logs
all assume a path that can be revalidated immediately before deletion.

Docker objects are daemon resources. Images, containers, volumes, networks, and
build cache entries need object IDs, object types, daemon state, reference
checks, and API error handling. Treating them as directories would violate the
trust model.

## Required Design Decisions

Before code, maintainers must choose:

- Whether Docker objects are represented in ActionPlan JSON or reported through
  a separate Docker-specific report.
- Whether the implementation uses a Rust Docker API client, the official
  `docker` CLI with array arguments, or both behind an abstraction.
- Whether Docker support is compiled by default or behind an optional feature.
- Which object kinds are eligible for the first report-only slice.
- How deletion evidence is recorded for non-restorable daemon operations.

## Execution Contract

If deletion is approved later:

- Use daemon/API or official CLI commands only; never direct-delete Docker
  storage paths.
- Pass arguments as arrays. Do not invoke a shell or interpolate command
  strings.
- Use explicit filters and object IDs. Do not run broad `docker system prune`
  as the primitive.
- Apply a bounded timeout and kill/reap child processes if CLI execution is
  used.
- Treat spawn failures, nonzero exits, malformed responses, timeout, permission
  denial, and object disappearance as explicit cleanup failures or skipped
  states.
- Do not use trash/graveyard restore modes for Docker daemon operations.
- Make `--dry-run` inspect/report only; it must not execute prune/delete
  commands.

## Proposed First Runtime Slice

After maintainer review, implement report-only Docker discovery:

- Probe Docker availability with a bounded timeout.
- Report daemon unavailable, permission denied, or timeout as explicit doctor
  or scan status.
- Inventory candidate reclaim categories without deletion:
  - build cache reclaim estimate
  - dangling image reclaim estimate
  - stopped container reclaim estimate
  - volume/network/named-resource report-only totals
- Back the inventory with mocked client tests; real daemon integration can be
  gated.

## Tests Required For Runtime Work

- Missing Docker binary or socket does not silently report success.
- Permission denied is explicit.
- Timeout is explicit.
- A resource that disappears before deletion is skipped/fails explicitly.
- Volumes and named resources are never selected by default.
- Direct Docker storage directory paths classify as blocked if scanned as
  ordinary filesystem paths.

## Verification

Spec-only verification:

```sh
test -s specs/GH159/product.md
test -s specs/GH159/tech.md
test -s specs/GH159/tasks.md
rg -n "Docker|docker|daemon|volume|ActionPlan|timeout|report-only|blocked" specs/GH159
cargo fmt -- --check
```

Future runtime verification:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95 cargo build --all-targets --all-features
rustup run 1.95 cargo test
```
