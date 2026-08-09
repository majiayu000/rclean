# GH374 Deterministic Parallel-Walk Warnings - Product Spec

## Linked Artifacts

- GitHub issue: `#374`
- Issue URL: `https://github.com/majiayu000/rclean/issues/374`
- Locale: `en-US`
- Route: `plan_first`

## Summary

Make scan warning order deterministic after the parallel filesystem walk. The
same warnings must appear in the same structured order for identical inputs,
independent of worker completion order.

## Problem

Candidate drafts are sorted after parallel workers merge, but warnings are
returned in mutex-acquisition order. JSON consumers and snapshot comparisons
can therefore observe ordering-only changes between equivalent scans, despite
the walker documenting deterministic output.

## Goals

- Sort walk warnings by variant, path, and error message.
- Reuse the sizing warning order already established in production.
- Preserve every warning without deduplication or text changes.
- Leave candidate/project order and all safety behavior unchanged.

## Non-Goals

- Do not change traversal, parallelism, warning generation, or error severity.
- Do not suppress repeated warnings.
- Do not change JSON schema or human warning formatting.

## Behavior Invariants

1. **B-001** equal warning sets produce equal ordered vectors.
2. **B-002** ordering is variant, then path, then message.
3. **B-003** warning count and values are preserved exactly.
4. **B-004** sizing and walking share one ordering implementation.
5. **B-005** scan classification, selection, and deletion are unchanged.

## Acceptance Criteria

- A regression inserts worker warnings in reverse order and observes canonical
  order from `WalkScratch::into_inner`.
- Existing sizing warning-order tests remain green.
- Full repository and Rust 1.95 gates pass.

