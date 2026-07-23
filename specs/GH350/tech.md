# Docker Report Probe Failure Honesty - Tech Spec

GitHub issue: `#350`
Product spec: `specs/GH350/product.md`

## Current Behavior

`src/docker.rs:231-249`:

```rust
pub fn print_report(report: &DockerReport) -> Result<(), RcleanError> {
    outln!("Docker: {}", report.status.human_label());
    match &report.status {
        DockerStatus::Available { server_version } => { ... }
        status => {
            outln!("Reason: {}", status_reason(status));
            outln!("No Docker cleanup resources reported.");   // failed probe
            return Ok(());
        }
    }

    if report.resources.is_empty() {
        outln!("No Docker cleanup resources reported.");        // real empty result
        return Ok(());
    }
    ...
```

The same sentence serves two incompatible meanings. Default timeout is
`src/docker.rs:14`, `Duration::from_secs(5)`, surfaced as
`--timeout` with `default_value = "5s"` (`src/cli.rs:85-87`).

## Design

### 1. Default timeout

Raise `DEFAULT_TIMEOUT` to 20s and change the clap `default_value` to
`"20s"` so `--help` and behavior agree. Both must move together: the
clap default applies on the CLI path, `DEFAULT_TIMEOUT` applies to
`DockerReportOptions::default()` and tests.

There is a **third** Docker timeout, which an earlier draft of this
spec got wrong by claiming `doctor --docker` routes through
`DockerReportOptions::default()`. It does not: `src/doctor.rs:72`
passed a hardcoded `Duration::from_secs(5)` straight to
`probe_for_doctor`, touching neither constant above. It is now named
`DOCTOR_PROBE_TIMEOUT` in `docker.rs` and deliberately stays at 5s —
that path only runs the fast `docker version` liveness probe, never
`system df`, and a diagnostic command should not sit for 20s against a
dead daemon. Naming it keeps every Docker timeout in the crate declared
in one module instead of leaving a literal in `doctor`.

20s is chosen as roughly 3x the measured 7.15s worst case here, leaving
headroom for a larger image set without making a genuinely hung daemon
feel unbounded.

### 2. Failure wording

Replace the failure-branch line with a message that describes rclean's
own failure rather than Docker's contents:

```rust
outln!(
    "Docker was not queried successfully, so nothing can be reported \
     about reclaimable space. Retry with a longer --timeout."
);
```

The successful-but-empty branch keeps "No Docker cleanup resources
reported." verbatim, so the two cases are textually distinguishable.

The `--timeout` pointer is included for every failure kind, not only
`TimedOut`. That is a deliberate simplification: it is accurate advice
for the timeout case and harmless for the others, and per-kind
remediation text would duplicate `status_reason`, which already names
the specific cause on the preceding line.

## Files Touched

| File | Change |
| --- | --- |
| `src/docker.rs` | `DEFAULT_TIMEOUT` 5s -> 20s; failure branch wording. |
| `src/cli.rs` | `--timeout` `default_value` `"5s"` -> `"20s"`. |
| `tests/docker_report_cli.rs` | Assertions for both branches. |

## Test Plan

The repository already has a fake-docker harness in
`tests/docker_report_cli.rs`; reuse it rather than adding a new one.

- Failed probe (fake docker that sleeps past the timeout, or an
  unavailable binary): stdout must NOT contain "No Docker cleanup
  resources reported.", must contain the not-queried wording, and exit
  must stay 3.
- Successful probe with no reclaimable resources: stdout MUST still
  contain "No Docker cleanup resources reported.", exit 0.
- The two messages must not be substrings of one another, so a
  `contains` assertion cannot pass for the wrong case.
- Unit assertion that the clap default and `DEFAULT_TIMEOUT` agree, so
  they cannot drift apart again.

## Risks

- **Trust model:** none. `docker report` is read-only and never selects
  or deletes; resource classification is untouched.
- **Behavior change / worst-case latency:** the timeout is a
  **per-command** bound, not a bound on the whole report. The success
  path runs five sequential Docker CLI calls — `docker version` in
  `probe`, then `system df`, `image ls`, `container ls`, `network ls`
  in `collect_resources` (`src/docker.rs:200-229`). A fully
  unreachable daemon still fails fast at the first probe (one bound,
  20s vs the previous 5s). But a *degraded* daemon that answers
  `docker version` and then hangs on each subsequent call can now take
  up to ~100s (5 x 20s) before the user sees anything, against ~25s
  before. That is a real 4x regression in the degraded case, accepted
  here because the previous default produced a confidently wrong
  answer on healthy machines — the common case — and `--timeout` lets
  a caller bound it lower.

  A single deadline for the whole report, rather than per-command,
  would remove the multiplication. That is an architectural change to
  the command runner and is deliberately out of scope for this issue;
  it is worth a follow-up.
- **JSON contract:** unchanged. Only `print_report` (human path) is
  edited.
