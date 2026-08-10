# GH381 First-Minute TUI Clarity - Tech Spec

## Linked Artifacts

- GitHub issue: `#381`
- Product spec: `specs/GH381/product.md`
- Tasks: `specs/GH381/tasks.md`
- Route: `plan_first`

```specrail-planned-changes
{
  "issue": 381,
  "complete": true,
  "paths": [
    "Cargo.lock",
    "Cargo.toml",
    "CHANGELOG.md",
    "specs/README.md",
    "specs/GH381/product.md",
    "specs/GH381/tech.md",
    "specs/GH381/tasks.md",
    "src/clean.rs",
    "src/clean/selection.rs",
    "src/clean/types.rs",
    "src/free.rs",
    "src/main.rs",
    "src/tui/mod.rs",
    "src/tui/select.rs",
    "src/tui/select/tests.rs"
  ],
  "spec_refs": [
    "specs/GH381/product.md",
    "specs/GH381/tech.md",
    "specs/GH381/tasks.md"
  ]
}
```

## Root Cause

The header and controls are single logical lines inside three-row bordered
blocks, leaving only one content row apiece. Ratatui clips both at the viewport
edge. The candidate list has no header and relies on glyph/color knowledge.

The TUI loop also collapses both terminal states into `Vec<SelectedCandidate>`:
`done` returns the selected vector and `cancelled` returns an empty vector. All
callers therefore execute their confirmed-empty rendering after a quit.

## Design

### Compact 80-column layout

Use two content rows for the summary and controls:

- summary row 1: roots;
- summary row 2: reclaimable, selected count/bytes, sort, filter;
- controls row 1: toggle, all-safe, explain, review, quit;
- controls row 2: search, sort, filter, movement, and a short review/confirm cue.

Add an explicit candidate column title and include the textual safety value in
every row. Color and glyphs remain supplementary signals. Use Unicode display
width and grapheme-aware truncation so CJK and emoji paths fit the same terminal
column budget as ASCII paths. When a narrow viewport cannot fit the fixed
metadata columns plus a useful path tail, collapse the row to the truncated path
tail instead of overflowing the viewport.

Pass a typed continuation value into the selector. Cleanup callers describe the
existing confirmation step, while standalone `tui` describes ActionPlan writing
and states that it does not clean.

### Cancellation contract

Add `SelectionOutcome::{Confirmed(Vec<SelectedCandidate>), Cancelled}` at the
clean-selection boundary. Text selection always returns `Confirmed`; only the
alternate-screen TUI can currently emit `Cancelled`.

The three TUI callers handle `Cancelled` immediately with exit code 3:

- `clean` returns before retaining/printing the scan report;
- standalone `tui` returns before writing an ActionPlan or message;
- `free --interactive` returns before plan rendering and confirmation.

No cancellation path reaches selection, confirmation, or deletion logic.
For clean with `--write-plan`, plan writing happens only after the selection
outcome is confirmed, so cancellation neither creates nor overwrites the path.

## Product-to-Test Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001/B-002 | two-row compact strings and taller blocks | 80-column line-length/label unit tests |
| B-003 | candidate title plus textual safety column | list title/item assertions |
| B-004 | `SelectionOutcome` from TUI loop | key/outcome unit tests |
| B-005 | early returns in three callers | match branches plus existing output tests |
| B-006 | confirmed vector unchanged | existing selector/clean/free suites |
| B-007 | text path wraps `Confirmed` | feature-combination tests |
| B-008 | display-width-aware row fitting | CJK/emoji row-width unit test |
| B-009 | typed selector continuation | clean/standalone controls unit tests |

## Verification

```sh
cargo test tui::select::tests
cargo test default_flow_tests
cargo test --no-default-features
cargo test --no-default-features --features tui
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95 cargo build --all-targets --all-features
rustup run 1.95 cargo test
```
