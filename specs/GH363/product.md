# Fail-Closed ActionPlan Replay Sizing - Product Spec

## Linked Artifact

- GitHub pull request: `#363`
- URL: `https://github.com/majiayu000/rclean/pull/363`
- Locale: `en-US`
- Route: `implement`
- Maintainer gate: approved in the active goal thread

`PR #363` is the stable GitHub identifier for this packet. No new public
security issue is created: `SECURITY.md` routes root-boundary and ActionPlan
trust-model details away from public issue intake, while the existing PR
already contains the reviewed change.

## Summary

ActionPlan replay must recompute each selected candidate's current size before
deletion, but it must fail before deletion when that size walk is incomplete.
The current PR adapter keeps only the readable partial byte count and discards
the structured walk or metadata warnings that prove the count is incomplete.

The fix preserves ordinary scan behavior, where partial sizes remain useful
when paired with explicit warnings, while making destructive replay strict.

## Problem

`dir_size` returns a `SizeOutcome` containing:

- readable bytes accumulated so far;
- the newest observed modification time;
- every `WalkError` or `MetadataError`.

That contract is appropriate for `scan`: a report can show partial data as
long as its warnings remain visible. PR #363 added a replay-only adapter that
returns only `SizeOutcome.bytes`. During `clean --plan`, an unreadable child can
therefore produce a partial number that is presented as the current complete
size and propagated into cleanup output, audit records, or graveyard metadata.

## Goals

- Keep PR #363's fail-closed canonical-root containment behavior.
- Keep current-on-disk byte recomputation during ActionPlan replay.
- Abort replay before deletion if current sizing produces any walk or metadata
  warning.
- Preserve all sizing failures in a contextual user-visible error.
- Keep ordinary scan behavior unchanged.

## Non-Goals

- Do not change the ActionPlan schema or serialized fields.
- Do not change deletion, symlink, hardlink, broad-root, TOCTOU, graveyard, or
  restore policy.
- Do not make ordinary scan sizing warnings fatal.
- Do not fall back to serialized plan bytes after a current sizing failure.
- Do not modify `docs/audit-2026-05-07.md`.

## Behavior Invariants

1. **B-001 Complete replay size:** a fully readable replay candidate receives
   its freshly recomputed current byte count.
2. **B-002 Fail closed:** any `WalkError` or `MetadataError` from replay sizing
   aborts `revalidate_selected` before the candidate can reach deletion.
3. **B-003 Complete error context:** the replay error identifies the candidate
   and preserves every sizing warning message.
4. **B-004 No stale fallback:** failed current sizing never reuses serialized
   plan bytes or claims bytes freed.
5. **B-005 Scan compatibility:** ordinary scan may still return partial bytes
   when it also returns the associated warnings.
6. **B-006 Existing trust gates:** canonical-root, tampered-safety,
   protected-data, symlink, hardlink, Docker-storage, runtime-path, and
   root-containment behavior remains unchanged.

## Acceptance Criteria

- Recorded RED evidence proves the prior adapter accepted 7 readable bytes
  after its size walk reported an unreadable descendant.
- A privilege-independent sizer test proves the real replay adapter rejects a
  deterministic metadata warning.
- A privilege-independent plan test injects both `WalkError` and
  `MetadataError`, then proves replay returns an error containing the candidate
  and both failing paths.
- A candidate that grows after plan creation still receives its current size.
- Existing sizer warning tests and all plan safety tests remain green.
- Full repository and Rust 1.95 gates pass.

## Boundary Checklist

| Boundary | Contract |
| --- | --- |
| Empty or missing candidate | Existing metadata error remains explicit. |
| Unreadable descendant | B-002/B-003 fail replay with complete context. |
| Multiple failures | B-003 preserves all warnings, not only the first. |
| Stale serialized bytes | B-001/B-004 recompute or fail; never fall back. |
| Ordinary scan | B-005 retains partial data plus warnings. |
| Deletion authorization | No deletion path is entered after sizing failure. |
| Schema compatibility | No schema or serialized field changes. |
| Safety policy | B-006 preserves all existing trust-model gates. |
