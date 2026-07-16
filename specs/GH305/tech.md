# Scan JSON Staleness Field Names - Tech Spec

## Linked Artifacts

- GitHub issue: `#305`
- Product spec: `specs/GH305/product.md`
- Tasks: `specs/GH305/tasks.md`
- Route: `write_spec`

## Root Cause Evidence

| Area | Evidence | Decision |
| --- | --- | --- |
| `src/model.rs::ScanReport` | `#[serde(rename_all = "camelCase")]` + `stale_after_days` | Public field is `staleAfterDays`. |
| `src/model.rs::Candidate` | `#[serde(rename_all = "camelCase")]` + documented `stalenessDays` | Public field is `stalenessDays`. |
| `README.md` Current Status | Calls `stale_after_days` a JSON field. | Replace only this token. |
| `README.md` Usage | Calls JSON fields `stale_after_days` / `staleness_days`. | Replace only these two tokens. |
| `README.md` user-rule section | Correct TOML `stale_after_days = 60` and JSON `stalenessDays`. | Preserve unchanged. |
| Historical specs | Contain design-time snake_case wording. | Preserve as historical context. |

Search of all GitHub issues/PRs and local specs found no open duplicate for this current README
contract mismatch.

## Design

Apply a token-only documentation correction in `README.md`:

1. Current Status: `stale_after_days` JSON field -> `staleAfterDays` JSON field.
2. Usage: report `stale_after_days` -> `staleAfterDays`; candidate `staleness_days` ->
   `stalenessDays`.

Do not alter surrounding prose. Keeping the diff token-only makes it possible to prove that no
configuration, historical documentation, schema, or runtime behavior changed.

## Product-to-Change Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001 | Current Status token replacement | exact positive and stale-phrase negative `rg` |
| B-002 | Usage token replacements | exact positive and stale-phrase negative `rg` |
| B-003 | no change to user-rule example | exact `stale_after_days = 60` check |
| B-004 | README-only token diff | name-only and word-diff review |
| B-005 | docs/full PR gates | focused commands + CI/SpecRail evidence |

## Planned Change Manifest

| Path | Change |
| --- | --- |
| `README.md` | Correct the two current JSON-field references to camelCase. |

No `src/`, test, historical spec, dependency, workflow, schema, CLI, or safety-policy file is
permitted in the implementation diff.

## Risks And Mitigations

- **Config key accidentally renamed:** assert the exact TOML example remains snake_case.
- **Broad historical rewrite:** require a one-file, token-only diff.
- **Field names chosen from memory:** derive them from the serde attributes and the existing correct
  README reference, not inference.
- **Runtime claim without evidence:** preserve runtime and rely on current CI; this PR changes docs
  only.

## Verification Plan

```sh
rg -n 'staleAfterDays.*JSON field' README.md
rg -n 'JSON output includes.*staleAfterDays.*stalenessDays' README.md
rg -n '^stale_after_days = 60$' README.md
! rg -n 'staleness reporting.*stale_after_days.*JSON field|JSON output includes.*stale_after_days|candidate.s staleness_days' README.md
git diff --check
git diff --name-only origin/main...HEAD
git diff --word-diff=porcelain origin/main...HEAD -- README.md
cargo fmt -- --check
```

Rust build/test may be skipped locally because B-004 forbids Rust changes; the three-platform and
MSRV CI checks remain required before merge.

## Rollback

Revert the README-only implementation commit. No runtime, schema, binary, dependency, or data
migration is involved.

## Human Gates

- Spec and implementation remain separate PRs.
- Implementation starts only after the Spec PR merges on latest `origin/main`.
- Merge only after current-head CI, independent review, zero unresolved review threads, clean merge
  state, valid signatures, SpecRail gate, and the user's standing authorization; never force push.
