# Empty Scan Guidance - Product Spec

GitHub issue: `#219`
Locale: `en-US`
Route: `implement`

## Summary

When an interactive or human-readable scan finds zero candidates, show a short
hint that points users toward `rclean scan --home` for toolchain caches and
`rclean scan --tmp` for temporary worktrees.

## Problem

The v0.2 CLI added broader entry points such as no-arg/TUI flows and global
cache scanning, but an empty local project scan still ends at "No cleanable
developer artifacts found." That is accurate, but it does not help users find
common rebuildable caches that live outside the current project tree.

## Goals

- Add one human-readable hint for empty candidate results.
- Keep machine-readable JSON output unchanged.
- Keep exit codes unchanged.
- Do not change scan selection, safety classification, cleanup behavior, or
  path scope.

## Non-Goals

- Do not automatically broaden a scan to `$HOME`, temp roots, or system roots.
- Do not add new cleanup rules.
- Do not change blocked/report-only behavior.
- Do not alter JSON fields or schema.

## Behavior

When human-readable output reports zero candidates, it should include:

```text
Hint: try `rclean scan --home` for toolchain caches or `rclean scan --tmp` for temp worktrees.
```

The hint appears only in human-readable output paths. `--json` output remains a
valid JSON document with no extra text.

## Acceptance Criteria

- Empty human-readable scan output includes the hint.
- Empty `scan --json` output does not include the hint.
- Exit code `3` for zero candidates is unchanged.
- No cleanup or ActionPlan behavior changes.
