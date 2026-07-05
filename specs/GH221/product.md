# Scan Throughput Trend Workflow - Product Spec

GitHub issue: `#221`
Locale: `en-US`
Route: `implement`

## Summary

Add a GitHub Actions workflow that runs the existing `scan_throughput`
Criterion benchmark on `main` pushes and uploads the Criterion report as an
artifact.

## Problem

`rclean` has a scan throughput benchmark, but performance trend evidence is not
produced automatically after changes land. Regressions can become visible only
when someone manually runs the benchmark.

## Goals

- Run the existing `scan_throughput` bench after pushes to `main`.
- Upload the Criterion report directory as a downloadable artifact.
- Keep the benchmark out of pull-request CI so it does not slow normal review.
- Add no pass/fail performance threshold yet.

## Non-Goals

- Do not add PR benchmark gates.
- Do not add trend thresholds or comments.
- Do not modify benchmark logic.
- Do not change runtime scan behavior.

## Behavior

The workflow runs on:

- `push` to `main`
- `workflow_dispatch` for manual verification

It executes:

```sh
cargo bench --bench scan_throughput
```

and uploads `target/criterion` as `scan-throughput-criterion`.

## Acceptance Criteria

- A `main` push run produces a downloadable Criterion artifact.
- Pull requests do not run the benchmark workflow.
- The workflow has no performance threshold gate.
- Runtime code and scan behavior are unchanged.
