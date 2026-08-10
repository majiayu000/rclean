# GH381 First-Minute TUI Clarity - Product Spec

## Linked Artifacts

- GitHub issue: `#381`
- Issue URL: `https://github.com/majiayu000/rclean/issues/381`
- Locale: `en-US`
- Route: `plan_first`

## Summary

Make the no-argument selector understandable in a standard 80x24 terminal and
make an explicit quit behave like cancellation rather than a confirmed empty
cleanup attempt.

## Problem

The selector currently spends one bordered row each on a long summary and a
long controls string. At 80 columns the reclaimable/selected totals and the
continue/quit controls are clipped. Candidate rows have visual glyphs but no
column labels, so a first-time user must already know the safety vocabulary.

Quitting the alternate-screen selector returns an empty vector. The clean flow
cannot distinguish that from confirming with nothing selected, so it prints the
full scan report and `Nothing selected.` after the user deliberately exits.

## Goals

- Keep totals and critical actions visible at 80x24.
- Label candidate state, category, size, staleness, name, and path.
- State the selector's real continuation: confirmation, dry-run preview,
  prompt-free cleanup, or ActionPlan writing.
- Preserve a distinct cancelled result through no-argument clean, standalone
  `tui`, and `free --interactive`.
- Exit cancellation before report, plan, confirmation, or deletion output.

## Non-Goals

- Do not change classification, selection eligibility, or delete modes.
- Do not change the text-selector fallback.
- Do not add onboarding persistence, a welcome wizard, themes, or animation.
- Do not change ActionPlan schema or output.
- Do not publish a release in this change.

## Behavior Invariants

1. **B-001** the 80-column selector keeps reclaimable and selected totals visible.
2. **B-002** toggle, explain, review, and quit actions fit at 80 columns.
3. **B-003** candidate columns align with row content and safety state is
   readable without color.
4. **B-004** `q`, Esc, and Ctrl-C produce an explicit cancelled outcome,
   including while search is active.
5. **B-005** cancellation exits before scan-table, plan, confirmation, and delete output.
6. **B-006** confirmed selection continues through the existing safety pipeline.
7. **B-007** non-TUI and alternate-screen fallback behavior is unchanged.
8. **B-008** candidate rows fit by terminal display columns for CJK, emoji, and
   other wide Unicode paths.
9. **B-009** Enter cues describe the caller's real next step: confirmation,
   dry-run preview, prompt-free cleanup, or standalone ActionPlan writing.

## Acceptance Criteria

- Unit tests bound every compact header/control line to the 80-column content
  width and assert the critical labels are present.
- Unit tests assert the candidate column title aligns with row content and
  explicit safety text.
- Unit tests assert wide Unicode rows fit the available terminal columns and
  preserve the distinguishing path tail.
- Unit tests distinguish cancelled from confirmed-empty outcomes.
- Unit tests assert cancellation does not create or overwrite `--write-plan`.
- Unit tests assert confirm, dry-run, prompt-free, and standalone `tui` flows
  show accurate continuation cues.
- Unit tests assert Ctrl-C cancels from both normal and search modes.
- Existing selector, clean, free, and feature-combination tests remain green.
- Full repository and Rust 1.95 gates pass.
