# Changelog Release Notes - Tech Spec

Product spec: `specs/GH217/product.md`
Tasks: `specs/GH217/tasks.md`
GitHub issue: `#217`

## Context

- Release automation lives in `.github/workflows/release.yml`.
- Release versions are tagged as `vX.Y.Z`.
- `CHANGELOG.md` uses `## X.Y.Z` headings.
- Repository preference is fail-closed for user-visible catalog/allowlist style
  errors; release notes extraction should likewise fail loudly instead of
  falling back to empty text.

## Proposed Changes

1. Add a small extraction step or script that maps `refs/tags/vX.Y.Z` to
   changelog heading `## X.Y.Z`.
2. Write the extracted body to a file.
3. Pass that file to the draft release creation step.
4. Fail if the heading is missing or extracted content is empty.
5. Add a dry-run/test path for success and missing-section failure.

## Safety And Compatibility

- Workflow-only change; no runtime cleanup behavior changes.
- Missing changelog content must be a hard failure.
- The human release publish gate remains unchanged.

## Validation

Focused:

```sh
rg -n 'CHANGELOG|body_path|body-file|release notes|missing changelog' .github/workflows/release.yml
```

Repository gate:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
