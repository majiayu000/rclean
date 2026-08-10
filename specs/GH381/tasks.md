# GH381 Tasks

## Linked Artifacts

- Issue: `#381`
- Product spec: `specs/GH381/product.md`
- Tech spec: `specs/GH381/tech.md`
- Route: `plan_first`

## Status

`approved_for_implementation` - reproduced in the live 80x24 no-argument TUI;
the change is output/cancellation-only and does not alter the trust model.

## Implementation Tasks

### SP381-T1 - Record the first-minute regressions

- [x] Bound compact summary/control lines at 80 columns.
- [x] Assert critical controls, column labels, and textual safety remain visible.
- [x] Assert quit differs from confirmed-empty selection.
- [x] Assert cancellation does not create or overwrite `--write-plan`.
- [x] Assert CJK/emoji rows fit by terminal display width.
- [x] Assert standalone `tui` describes ActionPlan writing rather than cleanup confirmation.
- [x] Assert dry-run and prompt-free cleanup cues describe their real continuation.
- [x] Assert Ctrl-C cancels while search is active.
- [x] Assert column headings use the same inset as list rows.
- Covers: B-001 through B-005, B-008, B-009.

### SP381-T2 - Make the selector responsive

- [x] Split summary and controls into two content rows.
- [x] Add a candidate column title and safety text.
- [x] Align the candidate column title with bordered/highlighted list rows.
- [x] Keep review/confirmation semantics visible.
- [x] Fit candidate labels and paths by terminal display width.
- [x] Render caller-specific confirm/dry-run/no-prompt/ActionPlan continuation cues.
- Covers: B-001, B-002, B-003, B-008, B-009.

### SP381-T3 - Preserve cancellation through callers

- [x] Return an explicit selection outcome from the TUI.
- [x] Exit clean, standalone TUI, and interactive free before post-selection output.
- [x] Handle Ctrl-C before normal/search-mode dispatch.
- [x] Leave the text fallback and confirmed-selection pipeline unchanged.
- Covers: B-004 through B-007.

### SP381-T4 - Document and verify

- [x] Update the changelog and packet index.
- [x] Re-run an 80x24 live smoke test.
- [x] Run focused, feature-matrix, full, release, and Rust 1.95 gates.
- Covers: B-001 through B-007.

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009}`
- Missing invariants: `none`
