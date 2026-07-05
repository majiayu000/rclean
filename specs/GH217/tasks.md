# GH217 Tasks

Issue: `#217`
Product spec: `specs/GH217/product.md`
Tech spec: `specs/GH217/tech.md`

## Status

- [x] `SP217-T001` Owner: `release` | Done when: release workflow extracts `## X.Y.Z` from `CHANGELOG.md` for tag `vX.Y.Z` | Verify: `rg -n 'CHANGELOG|refs/tags|GITHUB_REF_NAME' .github/workflows/release.yml`
- [x] `SP217-T002` Owner: `release` | Done when: extracted body is passed to draft release creation | Verify: `rg -n 'body|body_path|body-file' .github/workflows/release.yml`
- [x] `SP217-T003` Owner: `release` | Done when: missing or empty changelog section fails loudly | Verify: `rg -n 'missing|empty|exit 1|fail' .github/workflows/release.yml`
- [x] `SP217-T004` Owner: `tests` | Done when: dry-run/test coverage proves success and missing-section failure | Verify: implementation PR documents focused workflow/script test evidence

## Handoff Notes

- Do not add silent fallback release notes.
