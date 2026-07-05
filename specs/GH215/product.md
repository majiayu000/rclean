# Free Interactive Selector - Product Spec

GitHub issue: `#215`
Locale: `en-US`
Route: `implement`

## Summary

Add `rclean free <size> --interactive` so the computed free-space proposal opens
in the selector with the proposed safe candidates pre-selected. The user can
adjust the selection, then continue through the normal confirmation and
recoverable-delete flow.

## Problem

`rclean free <size>` can already compute a proposal and write an ActionPlan, but
the v0.2 design also called for handing that proposal to the interactive
selector. Without that path, users who want a target-sized cleanup still need to
review and edit indirectly through the plan flow.

## Goals

- Add an explicit `--interactive` mode to `free`.
- Require a TTY for interactive mode.
- Pre-select only candidates from the proposal that are safe and do not require
  sudo.
- Preserve selector guards for blocked and report-only candidates.
- Continue through the existing confirmation and recoverable-delete pipeline.

## Non-Goals

- Do not make `free` select caution, blocked, report-only, or sudo candidates by
  default.
- Do not change ActionPlan behavior for the existing plan path.
- Do not add background deletion or automatic broad scans.
- Do not weaken text selector or TUI safety rules.

## Behavior

`rclean free <size> --interactive`:

1. Scans using the same inputs as `free`.
2. Computes the same proposed candidate set as the plan path.
3. Opens the selector with those proposal rows pre-selected.
4. Lets the user adjust selection within existing safety constraints.
5. Runs the normal confirm and recoverable-delete cleanup flow.

If stdin/stdout are not interactive, the command returns an explicit error and
does not fall back to deletion, plan writing, or text output.

## Acceptance Criteria

- Selector state test proves proposal rows start pre-selected.
- E2E test proves non-TTY `--interactive` fails explicitly and deletes nothing.
- Existing `free` plan path behavior is unchanged.
- Blocked, report-only, caution without opt-in, and sudo candidates cannot be
  smuggled into pre-selection.
