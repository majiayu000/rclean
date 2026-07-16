# GH305 Tasks

## Linked Artifacts

- Issue: `#305`
- Product spec: `specs/GH305/product.md`
- Tech spec: `specs/GH305/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH305 Spec PR to merge.

## SpecRail Checklist

- [ ] `SP305-T1` | Owner: `docs` | Done when: Current Status and Usage use exact public camelCase staleness field names while the TOML key remains snake_case | Verify: focused positive/negative `rg` checks
- [ ] `SP305-T2` | Owner: `verification` | Done when: implementation is a README-only token diff and docs/full PR gates pass | Verify: diff scope, fmt, CI, SpecRail and PR gate

## Implementation Tasks

### SP305-T1 — Correct the public JSON field names

- Owner: `docs`
- Dependencies: merged GH305 Spec PR; latest `origin/main`
- Covers: B-001, B-002, B-003, B-004
- Change: replace the two current README JSON references with `staleAfterDays` and
  `stalenessDays`; preserve the `.rclean.toml` `stale_after_days` example and all surrounding text.
- Done when: Current Status and Usage name the shipped JSON fields exactly, the config example is
  unchanged, and the word diff contains only the three token replacements across two references.
- Verify:
  - `rg -n 'staleAfterDays.*JSON field' README.md`
  - ``rg -U -n 'JSON output includes\s+`staleAfterDays` and each candidate.s `stalenessDays`' README.md``
  - `rg -n '^stale_after_days = 60$' README.md`
  - ``! rg -U -n 'staleness reporting.*`stale_after_days` JSON field|JSON output includes\s+`stale_after_days`|candidate.s `staleness_days`' README.md``

## Verification And Handoff Tasks

### SP305-T2 — Prove token-only scope and merge readiness

- Owner: `verification`
- Dependencies: SP305-T1
- Covers: B-001, B-002, B-003, B-004, B-005
- Done when: diff is only `README.md`, word-diff review shows only the intended field-name tokens,
  `git diff --check` and fmt pass, and current-head CI/review/signature/merge-state/SpecRail gates
  are green.
- Verify:
  - `git diff --check`
  - `git diff --name-only origin/main...HEAD`
  - `git diff --word-diff=porcelain origin/main...HEAD -- README.md`
  - `cargo fmt -- --check`

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005}`
- Missing invariants: `none`

## Handoff Notes

- Implementation file: `README.md` only.
- Do not change runtime serialization to match the incorrect docs.
- Do not alter historical specs or the TOML configuration key.
- Merge only with fresh current-head gates under standing authorization; never force push.
