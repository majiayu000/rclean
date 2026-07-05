# README v0.2 Surface Refresh - Product Spec

GitHub issue: `#220`
Locale: `en-US`
Route: `implement`

## Summary

Refresh `README.md` so Current Status and Usage describe the shipped v0.2 CLI
surface, including default interactive flow, TUI explain, `free <size>`,
staleness, recovery summaries, shell completions, man page generation, and the
three install paths.

## Problem

The README still contains pre-v0.2 phrasing and usage examples. Users should be
able to copy documented commands against the v0.2.0 binary without hitting stale
or future-tense instructions.

## Goals

- Document no-arg default flow.
- Document TUI-by-default behavior and `?` explain.
- Document `free <size>`.
- Document staleness column and `stale_after_days`.
- Document recovery summaries.
- Document `completions` and `man`.
- Document cargo install, cargo-binstall, and Homebrew install paths.
- Remove stale "After public release" style phrasing.

## Non-Goals

- Do not change CLI behavior.
- Do not rewrite historical specs.
- Do not add new cleanup rules.
- Do not change release automation.

## Behavior

Every command shown in the refreshed README should be valid for the v0.2.0
binary. Where an example depends on release publication, the text should state
the real install path without implying it is future-only.

## Acceptance Criteria

- README Current Status and Usage reflect the v0.2 surface.
- Documented command lines copy-paste against the v0.2.0 binary.
- No stale "After public release" phrasing remains.
- Rust build/test can be skipped only if the implementation PR is docs-only and
  explains that choice after running focused docs checks and `cargo fmt -- --check`.
