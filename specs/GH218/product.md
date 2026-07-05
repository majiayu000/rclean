# Selector Sort And Category Filter - Product Spec

GitHub issue: `#218`
Locale: `en-US`
Route: `implement`
Depends on: `#215`

## Summary

Add selector controls for sorting and category filtering: `s` cycles sort modes,
`c` cycles category filters, the header shows active state, and fuzzy search
composes with both.

## Problem

Large candidate sets are hard to review with only fuzzy search and the default
ordering. Users need quick ways to focus on dependency directories, build
outputs, caches, and stale or risky candidates without losing selection state.

## Goals

- `s` cycles size descending, staleness descending, and risk ascending.
- `c` cycles all, deps, build, cache, and test categories.
- Header displays active sort and category filter.
- Fuzzy search composes with sort and category filter.
- Selection survives re-sort and re-filter by stable row identity.

## Non-Goals

- Do not add mouse support.
- Do not add new panes or theming.
- Do not change candidate classification or safety rules.
- Do not change cleanup execution or ActionPlan schema.

## Behavior

The selector starts with existing default ordering equivalent to size descending.
Pressing `s` advances through sort modes. Pressing `c` advances through category
filters. If search text is active, the visible rows are the intersection of the
category filter and fuzzy search, ordered by the active sort.

Selections are keyed by stable candidate identity, not visible row index, so a
selected candidate remains selected when filters or sorting change and becomes
visible again when the filter allows it.

## Acceptance Criteria

- Selector state tests cover sort cycling.
- Selector state tests cover category filter cycling.
- Selector state tests cover search plus category filter composition.
- Selector state tests prove selection stability across re-sort and re-filter.
