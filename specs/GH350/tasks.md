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

## Post-Review Follow-ups (PR #353)

- [x] R1: `src/doctor.rs:72` held a third hardcoded
      `Duration::from_secs(5)` that neither constant above covered, and
      the tech spec wrongly claimed `doctor --docker` routes through
      `DockerReportOptions::default()`. Named it
      `docker::DOCTOR_PROBE_TIMEOUT`, kept at 5s deliberately (that
      path only runs the fast `docker version` probe), and corrected
      the spec.
- [x] R2: The risk section claimed "20s instead of 5s". The timeout is
      per-command across five sequential calls, so a degraded daemon
      can now take ~100s vs ~25s. Documented the real worst case and
      recorded a single-report-deadline follow-up.
- [x] R3: "Retry with a longer --timeout." was printed for every
      failure kind, including a missing binary and permission denied,
      where it is useless advice. Now emitted only for `TimedOut`, with
      `docker_report_non_timeout_failure_does_not_suggest_timeout`
      covering it.

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
