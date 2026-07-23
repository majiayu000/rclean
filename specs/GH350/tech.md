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
`"20s"` so `--help` and behavior agree. Both constants must move
together — the clap default is what actually applies on the CLI path,
and `DEFAULT_TIMEOUT` is what applies to `DockerReportOptions::default()`
used by `doctor --docker` and tests.

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
- **Behavior change:** a hung daemon now blocks for 20s instead of 5s
  before reporting failure. Acceptable: the previous default returned a
  misleading answer quickly, which is worse than a slower correct one,
  and `--timeout` still lets a caller bound it lower.
- **JSON contract:** unchanged. Only `print_report` (human path) is
  edited.
