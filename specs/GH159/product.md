# GH159 product spec: Docker cleanup safety gates

## Summary

Add Docker and container cleanup coverage only behind daemon/API-aware safety
gates. Docker resources are not ordinary filesystem candidates, so the first
approved slice must be discovery/reporting and taxonomy before deletion.

## Problem

Developer machines accumulate Docker build cache, dangling images, stopped
containers, networks, and volumes. These can consume substantial disk space,
but Docker stores them behind daemon-managed metadata. Directly deleting
Docker storage directories can corrupt the daemon state or delete user data.

## Goals

- Define a Docker safety taxonomy before runtime implementation.
- Prefer inspect/report and ActionPlan review before destructive operations.
- Handle missing daemon, permission failures, disappearing objects, and
  timeouts explicitly.
- Keep volumes, named resources, tagged images, and running/in-use resources
  out of automatic deletion.
- Preserve `scan` as non-destructive and keep `--dry-run` from executing
  daemon prune commands.

## Non-goals

- No direct deletion under Docker storage directories.
- No `sudo`, service start/stop, or interactive prompt handling.
- No broad shell wrapper around `docker system prune`.
- No default deletion of volumes, named containers, named images, tagged
  images, networks, or ambiguous resources.
- No trash/graveyard restore semantics for daemon resources; Docker cleanup is
  non-restorable and must use a separate execution contract.

## Safety Taxonomy

| Tier | Docker resources |
| --- | --- |
| `safe` | None in the first implementation. |
| `caution` | Dangling images, stopped anonymous containers, and build cache entries only when the daemon proves they are unused. |
| `report-only` | Volumes, named resources, tagged images, networks, and reclaim estimates. |
| `blocked` | Running or in-use resources, permission-denied resources, unknown object types, Docker storage directories, and daemon states that cannot be revalidated. |

## Product Shape

The first code slice after maintainer review should report Docker reclaim
opportunities without deleting. A later deletion slice may use Docker daemon
APIs or official CLI commands with explicit object filters, bounded timeouts,
and immediate pre-delete revalidation.

## Done When

- Maintainer accepts the taxonomy and object model.
- The first runtime PR has mocked daemon/API tests for missing daemon,
  permission denied, timeout, stale object, and successful report-only output.
- README and native-tool policy explain that Docker cleanup is daemon/API
  backed and never direct filesystem deletion.
