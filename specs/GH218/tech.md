# Selector Sort And Category Filter - Tech Spec

Product spec: `specs/GH218/product.md`
Tasks: `specs/GH218/tasks.md`
GitHub issue: `#218`
Depends on: `#215`

## Context

- `src/tui/select.rs` owns row construction, visible row state, key handling,
  search, and selection behavior.
- GH215 is expected to introduce or harden stable candidate identity for
  pre-selection. GH218 should build on that identity instead of adding a second
  selection model.

## Proposed Changes

1. Add selector state enums for sort mode and category filter.
2. Apply visible row pipeline in this order:
   - category filter
   - fuzzy search
   - sort
3. Update key handling so `s` cycles sort and `c` cycles category.
4. Render active sort/filter in the header.
5. Store selected candidates by stable identity rather than visible index.
6. Add focused selector state tests.

## Safety And Compatibility

- UI-only selection state change; cleanup validation remains final authority.
- Existing blocked/report-only guards must remain unchanged.
- Search behavior should only narrow visible rows, not change eligibility.

## Validation

Focused:

```sh
cargo test selector_sort
cargo test selector_filter
```

Repository gate:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95 cargo build --all-targets --all-features
rustup run 1.95 cargo test
```
