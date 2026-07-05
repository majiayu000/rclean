# Post-Release Install Smoke Workflow - Product Spec

GitHub issue: `#216`
Locale: `en-US`
Route: `implement`

## Summary

Add a scheduled and manually dispatched smoke workflow that verifies the three
public install paths after release: cargo-binstall, Homebrew, and cargo install.

## Problem

`rclean` now has multiple distribution paths. A release can appear successful
while one install path is broken. The release checklist needs live evidence that
users can install and run the published binary or crate.

## Goals

- Add a workflow with `workflow_dispatch` and a weekly schedule.
- Use separate jobs for binstall, Homebrew, and cargo install.
- Run `rclean --version` after each install.
- Let failures be visible independently so one broken path does not mask
  another.

## Non-Goals

- Do not publish releases or crates automatically.
- Do not add this to PR CI.
- Do not make expected pre-publication red runs block ordinary development.
- Do not change package metadata or release artifacts unless required by the
  smoke workflow.

## Behavior

The workflow verifies:

- `cargo binstall rclean-cli --no-confirm` on Ubuntu and macOS.
- `brew install majiayu000/rclean/rclean` on macOS.
- `cargo install rclean-cli --locked` on Ubuntu.

Each job ends with `rclean --version`.

## Acceptance Criteria

- The workflow can be run manually.
- The workflow runs weekly.
- Binstall, Homebrew, and cargo install are separate jobs.
- First green post-release run can be cited as release verification evidence.
