# Empty Scan Guidance - Tech Spec

Product spec: `specs/GH219/product.md`
Tasks: `specs/GH219/tasks.md`
GitHub issue: `#219`

## Context

- `src/main.rs` runs `scan::scan`, then calls `output::print_json` for
  `--json` or `output::print_table` for human-readable scan output.
- `output::print_table` already has a zero-project branch that prints
  "No cleanable developer artifacts found."
- Zero-candidate scans return exit code `3`; that behavior is controlled after
  output rendering in `src/main.rs`.

## Proposed Changes

1. Add a small output helper for the empty-result hint.
2. Call the helper from the human-readable empty result path in
   `output::print_table`.
3. Add CLI regression tests:
   - human-readable empty scan includes the hint and exits `3`
   - `scan --json` on an empty directory exits `3`, remains JSON-only, and does
     not include the hint

## Safety And Compatibility

- This is output-only. It does not change candidate discovery, safety labels,
  ActionPlan generation, cleanup selection, deletion, or root guards.
- JSON output is intentionally untouched so automation remains stable.
- The exit code for zero candidates remains `3`.

## Validation

Focused:

```sh
cargo test --test cli empty_scan
```

Repository gate:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
