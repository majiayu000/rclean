# GH218 Tasks

Issue: `#218`
Product spec: `specs/GH218/product.md`
Tech spec: `specs/GH218/tech.md`
Depends on: `#215`

## Status

- [x] `SP218-T001` Owner: `selector` | Done when: selector has sort modes size desc, staleness desc, and risk asc | Verify: `cargo test selector_sort`
- [x] `SP218-T002` Owner: `selector` | Done when: selector has category filters all, deps, build, cache, and test | Verify: `cargo test selector_filter`
- [x] `SP218-T003` Owner: `selector` | Done when: fuzzy search composes with category filter and active sort | Verify: `cargo test selector_filter`
- [x] `SP218-T004` Owner: `selector` | Done when: header displays active sort and filter | Verify: `cargo test selector_header`
- [x] `SP218-T005` Owner: `safety` | Done when: selection survives re-sort/re-filter by stable candidate identity | Verify: `cargo test selector_selection_stability`

## Handoff Notes

- Implement after GH215 to reuse its stable identity work.
- Do not change cleanup validation or candidate classification.
