# GH373 Watch Interval Semantics - Product Spec

## Linked Artifacts

- GitHub issue: `#373`
- Issue URL: `https://github.com/majiayu000/rclean/issues/373`
- Locale: `en-US`
- Route: `plan_first`

## Summary

`rclean watch --every 5m` promises a five-minute polling interval but currently
routes through the scan-age parser, where `m` deliberately means a 30-day
month. Correct watch intervals to use short-duration semantics without changing
the established meaning of scan-age flags such as `--older-than`.

## Problem

Watch fallback calls `parse_duration`, the parser shared with staleness and age
filters. That domain supports seconds, hours, days, weeks, months, and years, so
`5m` becomes 150 days. The watch help text uses `5m` as its canonical interval
example, making the implementation contradict the CLI contract.

## Goals

- Interpret watch `m` as minutes.
- Reject invalid watch intervals during CLI parsing, before any scan starts.
- Preserve scan-age month semantics unchanged.
- Reuse the repository's existing short-duration parser and error contract.

## Non-Goals

- Do not change watcher event handling, polling degradation, or output.
- Do not add milliseconds or fractional durations.
- Do not change scan, cleanup, selection, ActionPlan, or safety policy.
- Do not rename `--every` or change its 60-second default.

## Behavior Invariants

1. **B-001** `watch --every 5m` resolves to exactly 300 seconds.
2. **B-002** `watch --every 1h` resolves to exactly 3,600 seconds.
3. **B-003** invalid watch units fail at argument parsing before scan work.
4. **B-004** the default watch interval remains 60 seconds.
5. **B-005** scan-age `5m` remains five 30-day months.
6. **B-006** no trust-model, cleanup, or ActionPlan behavior changes.

## Acceptance Criteria

- A typed CLI regression proves the parsed `WatchArgs` interval is 300 seconds.
- A CLI failure regression proves an age-only unit such as `5d` is rejected for
  watch without starting a scan.
- Existing scan-age and timeout parser tests remain green.
- Full repository and Rust 1.95 gates pass.

