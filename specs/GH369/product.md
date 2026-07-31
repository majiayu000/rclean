# GH369 Fail-Closed Safety Corrections - Product Spec

## Linked Artifacts

- GitHub issue: `#369`
- Issue URL: `https://github.com/majiayu000/rclean/issues/369`
- Implementation history: PR `#363`
- Implementation URL: `https://github.com/majiayu000/rclean/pull/363`
- Locale: `en-US`
- Route: `plan_first`

## Summary

Issue #369 is the stable identifier for the complete safety design implemented
by PR #363. That implementation closed two fail-open gaps:

1. path-safety containment checks now block a candidate when either the scan
   root or candidate cannot be canonicalized;
2. ActionPlan replay now aborts before deletion when current-size
   recomputation is incomplete.

The implementation preserves ordinary scan behavior, where partial sizes remain
useful when paired with explicit warnings, while making safety classification
and destructive replay fail closed.

## Problem

### H5 — canonical-root containment

`apply_path_safety` previously converted canonicalization results to `Option`
and performed containment only when both conversions succeeded. If either the
non-dot root or candidate could not be canonicalized, the containment check was
silently skipped and the candidate retained its earlier safety classification.

### H8 — ActionPlan replay sizing

`dir_size` returns a `SizeOutcome` containing:

- readable bytes accumulated so far;
- the newest observed modification time;
- every `WalkError` or `MetadataError`.

That contract is appropriate for `scan`: a report can show partial data as
long as its warnings remain visible. The initial replay-only adapter returned
only `SizeOutcome.bytes`. During `clean --plan`, an unreadable child could
therefore produce a partial number presented as the current complete size and
propagated into cleanup output, audit records, or graveyard metadata.

## Goals

- Block non-dot-root candidates when root or candidate canonicalization fails.
- Preserve an explicit user-visible canonicalization warning.
- Block candidates that resolve outside the canonical scan root.
- Keep current-on-disk byte recomputation during ActionPlan replay.
- Abort replay before deletion if current sizing produces any walk or metadata
  warning.
- Preserve all sizing failures in a contextual user-visible error.
- Keep ordinary scan behavior unchanged.

## Non-Goals

- Do not change production code as part of issue #369; this packet records the
  complete behavior already shipped by PR #363.
- Do not change the ActionPlan schema or serialized fields.
- Do not change deletion, symlink, hardlink, broad-root, TOCTOU, graveyard, or
  restore policy.
- Do not make ordinary scan sizing warnings fatal.
- Do not fall back to serialized plan bytes after a current sizing failure.
- Do not modify `docs/audit-2026-05-07.md`.

## Behavior Invariants

1. **B-001 Canonicalization failure blocks:** for a non-dot root, failure to
   canonicalize either the root or candidate sets `Safety::Blocked` and adds a
   `failed to canonicalize path for containment check` warning.
2. **B-002 Escaped candidate blocks:** a candidate resolving outside the
   canonical scan root is blocked with an explicit containment warning.
3. **B-003 Complete replay size:** a fully readable replay candidate receives
   its freshly recomputed current byte count.
4. **B-004 Replay fails closed:** any `WalkError` or `MetadataError` from replay
   sizing aborts `revalidate_selected` before deletion.
5. **B-005 Complete error context:** the replay error identifies the candidate
   and preserves every sizing warning message.
6. **B-006 No stale fallback:** failed current sizing never reuses serialized
   plan bytes or claims bytes freed.
7. **B-007 Scan compatibility:** ordinary scan may still return partial bytes
   when it also returns the associated warnings.
8. **B-008 Existing trust gates:** tampered-safety, protected-data, symlink,
   hardlink, Docker-storage, runtime-path, and other root-containment behavior
   remains unchanged.

## Acceptance Criteria

- `canonicalize_failure_blocks_candidate` proves a missing candidate under a
  non-dot root becomes blocked with a canonicalization warning.
- `candidate_resolving_outside_root_is_blocked` proves a resolved candidate
  outside the scan root becomes blocked with a containment warning.
- Recorded RED evidence proves the prior adapter accepted 7 readable bytes
  after its size walk reported an unreadable descendant.
- A privilege-independent sizer test proves the real replay adapter rejects a
  deterministic metadata warning.
- A privilege-independent plan test injects both `WalkError` and
  `MetadataError`, then proves replay returns an error containing the candidate
  and both failing paths.
- A candidate that grows after plan creation still receives its current size.
- Existing sizer warning tests and all plan safety tests remain green.
- Full repository and Rust 1.95 gates pass for the historical implementation.
- Issue #369 preserves exactly 50 packet directories and 150 packet files.

## Boundary Checklist

| Boundary | Contract |
| --- | --- |
| Root or candidate canonicalization failure | B-001 blocks with explicit context. |
| Candidate resolves outside root | B-002 blocks with explicit context. |
| Empty or missing replay candidate | Existing metadata error remains explicit. |
| Unreadable descendant | B-004/B-005 fail replay with complete context. |
| Multiple failures | B-005 preserves all warnings, not only the first. |
| Stale serialized bytes | B-003/B-006 recompute or fail; never fall back. |
| Ordinary scan | B-007 retains partial data plus warnings. |
| Deletion authorization | No deletion path is entered after sizing failure. |
| Schema compatibility | No schema or serialized field changes. |
| Other safety policy | B-008 preserves all existing trust-model gates. |
