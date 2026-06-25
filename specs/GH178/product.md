# Adopt SpecRail Agent Workflow - Product Spec

GitHub issue: `#178`
Locale: `en-US`
Route: `write_spec`

## Summary

Adopt a minimal SpecRail-style workflow in `rclean` so agents can join the
repository, choose the right route, create linked specs for substantial work,
and verify changes with the repo's real Rust gates without replacing existing
project documentation.

## Problem

`rclean` already has strong human-facing documentation in `README.md`,
`CONTRIBUTING.md`, `SECURITY.md`, and historical specs under `docs/specs/`.
Agents do not currently have a top-level router that explains which files to
load first, when to require a spec, which trust-model changes need maintainer
review, or which GitHub artifacts should link issue/spec/PR work.

## Goals

- Add a short top-level `AGENTS.md` for repository-wide agent routing.
- Add a repo-local skill for new or changed cleanup rules.
- Extend GitHub issue templates for false positives, safety concerns, and
  feature requests.
- Add a PR template that asks for linked work, trust-model impact, and fresh
  verification commands.
- Add this first SpecRail packet under `specs/GH178/`.
- Align stale contribution guidance with the current Rust MSRV and CI commands.

## Non-Goals

- Do not migrate existing `docs/specs/` files.
- Do not change runtime code, cleanup rules, CLI behavior, or release process.
- Do not vendor the whole SpecRail pack into `rclean`.
- Do not automate final approval, merge, force push, or security disclosure.

## Users

- Maintainers reviewing issue/spec/PR readiness.
- Agents implementing rule, safety, docs, and workflow changes.
- Contributors filing bugs or proposed cleanup rule changes.

## Behavior

Agents should:

1. Read `AGENTS.md` first.
2. Search existing issues, PRs, specs, and drafts before creating new work.
3. Use direct docs-only flow for small wording changes.
4. Use linked issue plus `specs/GH<number>/product.md`, `tech.md`, and
   `tasks.md` for substantial behavior, rule, safety, or multi-module work.
5. Stop for maintainer review on security/private/trust-model changes.
6. Run the repo's real verification commands instead of generic placeholders.

## Acceptance Criteria

- `AGENTS.md` exists and points to README, CONTRIBUTING, SECURITY, historical
  specs, SpecRail packet paths, and verification commands.
- `.agents/skills/add-rclean-rule/SKILL.md` exists and captures rule-specific
  preflight, implementation map, tests, and stop conditions.
- `.github/ISSUE_TEMPLATE/` contains false-positive, safety-concern, and
  feature-request templates with SpecRail-compatible fields.
- `.github/PULL_REQUEST_TEMPLATE.md` contains linked-work, trust-model, and
  verification sections.
- `specs/GH178/product.md`, `tech.md`, and `tasks.md` exist and link issue
  `#178`.
- CONTRIBUTING MSRV and local verification commands match `Cargo.toml` and CI.

## Done When

The adoption PR runs the relevant docs/workflow verification commands, links
issue `#178`, and leaves `rclean` runtime behavior unchanged.
