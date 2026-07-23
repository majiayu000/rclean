# Docker Report Probe Failure Honesty - Product Spec

GitHub issue: `#350`
Locale: `en-US`
Route: `implement`

## Summary

Stop `rclean docker report` from failing by default on a healthy Docker
daemon, and stop rendering a failed probe with the same sentence used
for a successful empty result.

## Problem

Two defects, the second more serious than the first.

### 1. The default timeout fails on an ordinary machine

`docker system df` walks every image, container, and volume layer.
Measured at 7.15s on a healthy daemon (Docker 29.5.3), against a
default of `5s`:

```
$ rclean docker report
Docker: timed-out
Reason: docker system df --format {{json .}} timed out after 5000ms
No Docker cleanup resources reported.

$ docker info --format '{{.ServerVersion}}'
29.5.3                                # daemon is fine

$ time docker system df --format '{{json .}}'
...  7.15s total                      # over the 5s limit
```

`--timeout` exists and works, but a default that fails on a normal
developer machine makes the user discover a flag before they can get an
answer at all.

### 2. A failed probe asserts an empty result

`src/docker.rs:241` (probe failed) and `src/docker.rs:247` (queried
successfully, nothing reclaimable) print the identical sentence:

```
No Docker cleanup resources reported.
```

So the failure output reads:

```
Docker: timed-out
Reason: docker system df --format {{json .}} timed out after 5000ms
No Docker cleanup resources reported.
```

"No cleanup resources reported" states a fact about Docker. The truth
was "rclean failed to look" — on this machine, while ~8 GB was
actually reclaimable. Someone reading the table concludes Docker is
clean. This is the failure mode AGENTS.md names directly: *"Do not
silently degrade user-visible errors into warnings with fallback
data."*

The exit code is already correct (3) and the JSON output already
distinguishes the case via `status.kind`. Only the human rendering
conflates them.

## Goals

- `rclean docker report` succeeds by default on a healthy daemon whose
  `docker system df` takes several seconds.
- The human output for a failed probe never asserts that there is
  nothing to clean.
- A successful query that genuinely finds nothing still says so
  plainly.

## Non-Goals

- Do not change the JSON schema; `status.kind` already distinguishes
  every case and consumers depend on it.
- Do not change exit codes; a failed probe already exits 3.
- Do not change Docker resource classification or safety labels.
- Do not add retries, caching, or a progress indicator.
- Do not delete anything; `docker report` stays read-only.

## User-Visible Behavior

Failed probe, before:

```
Docker: timed-out
Reason: docker system df --format {{json .}} timed out after 5000ms
No Docker cleanup resources reported.
```

after:

```
Docker: timed-out
Reason: docker system df --format {{json .}} timed out after 20000ms
Docker was not queried successfully, so nothing can be reported about
reclaimable space. Retry with a longer --timeout.
```

Successful query with nothing reclaimable is unchanged — it renders the
taxonomy with zero counts:

```
Docker: available
Server: 29.5.3
Resource                     Safety          Count           Size Reclaimable
docker.dangling_images       caution             0              - -
docker.stopped_containers    report-only         0              - -
docker.networks              report-only         0              - -
```

## Acceptance Criteria

1. The default timeout is high enough that a daemon taking ~7s to run
   `docker system df` reports successfully without passing `--timeout`.
2. A failed probe (timeout, unavailable, permission denied, error)
   never prints "No Docker cleanup resources reported."
3. The failure message states that the query failed and points at
   `--timeout`.
4. A successful probe that finds nothing reclaimable renders its
   zero-count table and never carries the failure wording.
5. Exit codes are unchanged: 0 when available, 3 otherwise.
6. The JSON output is byte-for-byte unchanged for both cases.

## Implementation Note

`collect_resources` always emits its fixed taxonomy, so a successful
probe renders rows with zero counts rather than collapsing into a
single sentence. That makes the `resources.is_empty()` branch in
`print_report` — the original home of "No Docker cleanup resources
reported." — effectively unreachable on the success path. This change
leaves that branch alone rather than removing it: deleting a reachable-
looking arm is a wider behavioral question than this issue, and the
sentence is now used by exactly one branch instead of two, which is the
ambiguity being fixed. Worth a separate look.
