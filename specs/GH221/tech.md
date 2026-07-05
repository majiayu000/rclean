# Scan Throughput Trend Workflow - Tech Spec

Product spec: `specs/GH221/product.md`
Tasks: `specs/GH221/tasks.md`
GitHub issue: `#221`

## Context

- The existing benchmark lives at `benches/scan_throughput.rs`.
- `.github/workflows/ci.yml` runs on both `push` and `pull_request`; adding the
  benchmark there would slow PR CI.
- Criterion writes reports under `target/criterion`.

## Proposed Changes

1. Add `.github/workflows/benchmarks.yml`.
2. Trigger it on `push` to `main` and `workflow_dispatch`, with no
   `pull_request` trigger.
3. Install stable Rust, run `cargo bench --bench scan_throughput`, and upload
   `target/criterion` with `if-no-files-found: error`.

## Safety And Compatibility

- This is CI-only. It does not change cleanup rules, scan output, selection,
  deletion, ActionPlan behavior, or safety policy.
- The workflow is visibility-only; Criterion measurement changes fail only if
  the benchmark command itself fails, not because a trend threshold is missed.

## Validation

Focused:

```sh
test -s .github/workflows/benchmarks.yml
rg -n 'pull_request' .github/workflows/benchmarks.yml
rg -n 'cargo bench --bench scan_throughput|target/criterion' .github/workflows/benchmarks.yml
```

Repository gate:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
