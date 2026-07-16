# GH281 Tasks

## Linked Artifacts

- Issue: `#281`
- Product spec: `specs/GH281/product.md`
- Tech spec: `specs/GH281/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH281 Spec PR to merge.

## Implementation Tasks

### SP281-T1 — Capture deterministic closed-pipe baselines

- Owner: `implementation`
- Dependencies: merged GH281 Spec PR; latest `origin/main`
- Covers: B-001, B-003, B-011
- Change: add std anonymous-pipe E2E for `rules` and `doctor`, plus structured error/status unit cases,
  before production changes.
- Done when: current main fails with panic/101 evidence for both commands and no test uses shell timing.
- Verify: exact failing focused tests and stderr assertions.

### SP281-T2 — Add the fallible stdout boundary

- Owner: `implementation`
- Dependencies: SP281-T1
- Covers: B-002, B-003, B-004, B-005, B-010, B-012
- Change: introduce the stdio writer/classifier, route direct report/JSON/generator output through it,
  and finish each command with its computed semantic status.
- Done when: direct stdout I/O errors remain typed, only BrokenPipe is quiet, JSON serializes first,
  and normal output assertions are unchanged.
- Verify: focused pipe tests, existing output suites and source search for undeclared non-interactive
  `println!` sites.

### SP281-T3 — Preserve mutating-command safety order

- Owner: `implementation`
- Dependencies: SP281-T2
- Covers: B-005, B-006, B-007, B-008, B-009, B-012
- Change: propagate free/stamp/watch/clean output errors without moving plan, stamp or delete calls;
  stop clean before deletion on pre-delete BrokenPipe and preserve post-delete 0/1 status.
- Done when: a closed pre-delete clean pipe leaves its target intact, post-delete status tests retain
  supplied failures, and watch exits rather than looping after output closure.
- Verify: pipe E2E, existing free/stamp/watch/clean suites, and control-flow diff review.

### SP281-T4 — Document the pipeline contract

- Owner: `implementation`
- Dependencies: SP281-T2, SP281-T3
- Covers: B-003, B-005, B-006, B-007, B-010, B-012
- Change: document quiet BrokenPipe handling, semantic status retention and pre-delete stop behavior.
- Done when: README and architecture match the code without promising ignored non-BrokenPipe errors.
- Verify: docs/source comparison and `cargo fmt -- --check`.

## Verification And Handoff Tasks

### SP281-T5 — Full gate, VibeGuard and SpecRail audit

- Owner: `verification`
- Dependencies: SP281-T1, SP281-T2, SP281-T3, SP281-T4
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012
- Done when: focused, stable, release, exact MSRV, VibeGuard and current-head PR gates pass with no
  extra path, output drift, panic matching, swallowed non-BrokenPipe error or safety-order change.
- Verify:
  - `cargo test --test cli pipe_output -- --nocapture`
  - `cargo test --test cli -- --nocapture`
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - all installed VibeGuard Rust guards

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009, B-010, B-011, B-012}`
- Missing invariants: `none`

## Handoff Notes

- Use `std::io::pipe()`; do not add a pipe crate.
- Never catch a panic or match an OS error string.
- Preserve a known command status instead of replacing every BrokenPipe with 0.
- A pre-delete closed pipe is a stop signal, never permission to continue cleanup.
- Do not alter terminal-only TUI/selection rendering or stderr diagnostics.
