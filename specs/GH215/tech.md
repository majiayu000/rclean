# Free Interactive Selector - Tech Spec

Product spec: `specs/GH215/product.md`
Tasks: `specs/GH215/tasks.md`
GitHub issue: `#215`

## Context

- `src/free.rs` computes a proposal from `ScanReport` and currently supports the
  plan-oriented flow.
- `src/tui/select.rs` owns selector state and selected row behavior.
- `src/clean/selection.rs` owns text selection and safety filtering.
- Cleanup execution must continue through existing clean validation and
  recoverable delete paths.

## Proposed Changes

1. Extend `FreeArgs` with `--interactive`.
2. Add a TTY check before interactive cleanup begins.
3. Represent pre-selected rows by stable candidate identity rather than display
   index.
4. Thread the proposal selection into TUI and text selector state without
   allowing blocked/report-only/sudo candidates.
5. Reuse the normal cleanup execution path after the user confirms the adjusted
   selection.
6. Add regression tests for pre-selection, non-TTY failure, and unchanged plan
   path behavior.

## Safety And Compatibility

- Pre-selection is a UI convenience only; it must not bypass final cleanup
  validation.
- The proposal must remain limited to safe non-sudo candidates.
- Existing ActionPlan output and `free` non-interactive behavior must remain
  stable.

## Validation

Focused:

```sh
cargo test free_interactive
cargo test --test cli free_interactive
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
