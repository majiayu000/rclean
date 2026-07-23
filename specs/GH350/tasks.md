# Docker Report Probe Failure Honesty - Tasks

GitHub issue: `#350`
Product spec: `specs/GH350/product.md`
Tech spec: `specs/GH350/tech.md`

## Tasks

- [x] T1: Raise `DEFAULT_TIMEOUT` in `src/docker.rs` from 5s to 20s.
- [x] T2: Raise the `--timeout` clap `default_value` in `src/cli.rs` to
      `"20s"` so `--help` matches actual behavior.
- [x] T3: Guard against the two defaults drifting apart again
      (`docker::tests::default_timeout_matches_cli_default` parses the
      clap default and asserts equality with `DEFAULT_TIMEOUT`).
- [x] T4: Replace the failure-branch sentence in `print_report` so it
      describes the failed query instead of asserting an empty result.
- [x] T5: Leave the successful path untouched.
- [x] T6: Test — failed probe does not print "No Docker cleanup
      resources reported.", prints the not-queried wording, exits 3.
- [x] T7: Test — successful probe with nothing reclaimable renders its
      zero-count table, never carries the failure wording, exits 0.
- [x] T8: CHANGELOG entry.
- [x] T9: Verification gate — `cargo fmt -- --check`, `cargo clippy
      --all-targets --all-features -- -D warnings`, `cargo test`, plus
      a manual `docker report` run on the real daemon.

## Deviation From The Tech Spec

T7 was specified as "successful empty probe still prints 'No Docker
cleanup resources reported.'" That premise was wrong, and the first
version of the test failed against it. `collect_resources` always emits
its fixed taxonomy, so a successful probe renders zero-count rows and
never reaches the `resources.is_empty()` branch. The test now asserts
the behavior that actually exists rather than the behavior the spec
assumed; see the Implementation Note in `product.md`.

## Acceptance Mapping

| Acceptance criterion | Tasks |
| --- | --- |
| 1. Default works on a ~7s daemon | T1, T2, T3, T9 |
| 2. Failure never claims an empty result | T4, T6 |
| 3. Failure points at `--timeout` | T4, T6 |
| 4. Real empty result still says so | T5, T7 |
| 5. Exit codes unchanged | T6, T7 |
| 6. JSON unchanged | T4 (human path only) |
