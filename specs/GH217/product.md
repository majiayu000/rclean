# Changelog Release Notes - Product Spec

GitHub issue: `#217`
Locale: `en-US`
Route: `implement`

## Summary

Use the matching `CHANGELOG.md` version section as the draft GitHub release body
in the release workflow, and fail loudly if the section is missing.

## Problem

Release notes should match the repository changelog. Copy-paste release notes
can drift, and an empty generated body would hide the problem until a human
notices it during release publication.

## Goals

- Extract the `## X.Y.Z` section matching the tag version.
- Pass the extracted text as the draft release body.
- Fail with a clear error if the changelog section is missing or empty.
- Cover the extraction path with a dry-run or testable workflow step.

## Non-Goals

- Do not auto-publish GitHub releases.
- Do not auto-publish crates.
- Do not rewrite changelog format beyond what extraction requires.
- Do not silently fall back to empty or generic notes.

## Behavior

For tag `vX.Y.Z`, the release workflow finds the `## X.Y.Z` section in
`CHANGELOG.md`, extracts content until the next `## ` heading or end of file,
and uses that content as the draft release body.

If the section is missing or empty, the release job fails with a clear message.

## Acceptance Criteria

- Draft releases use the matching changelog section as body text.
- Missing changelog section fails loudly.
- Extraction is covered by a dry-run job, script test, or equivalent workflow
  verification.
