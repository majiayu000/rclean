# Contributing to rclean

Thanks for taking the time to contribute. `rclean` is a small CLI with
a deliberately narrow scope — these notes capture the working patterns
so a PR can land cleanly the first time.

## Scope and product trust model

`rclean` deletes files. The trust model **is** the product:

- `scan` never deletes.
- `clean --all` selects only `safe` candidates unless `--include-caution`
  is passed. **Blocked candidates are never selected**, even with
  `--include-blocked`.
- Plan-based cleanup re-validates every path against the live filesystem
  before deleting, refuses to follow new symlinks, and rejects plans
  whose roots have changed shape since the scan.

A change that weakens any of these guarantees needs an explicit
discussion in the PR before code, ideally as a SPEC update under
`docs/specs/` first. See [`SECURITY.md`](SECURITY.md) for the threat
model and reporting workflow.

## Toolchain

- **Edition**: Rust 2024.
- **MSRV**: `rust-version = "1.88"` in `Cargo.toml`. CI verifies the
  pinned MSRV on Ubuntu; PRs that need a newer toolchain must bump
  `rust-version` and explain why in the commit message.
- **OS coverage**: CI runs `fmt`, `clippy`, `test`, and a release build
  on Ubuntu, macOS, and Windows. Platform-specific code must be gated
  with `cfg(unix)` / `cfg(windows)` and have at least one
  platform-gated test in `tests/cross_platform.rs`.

## Local verification

Run these before opening a PR — they mirror what CI runs:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
```

If your change is performance-sensitive, also run the benchmark suite
(see `benches/` and `docs/perf/`) and include the before/after numbers
in the PR body.

## PR shape

- **Small and focused.** One concern per PR. If a refactor and a fix
  are tangled, land the refactor first as a pure-rename PR, then the
  fix.
- **Conventional commits.** Existing history uses `feat:`, `fix:`,
  `perf:`, `refactor:`, `test:`, `docs:`, `chore:` prefixes with an
  optional scope (e.g. `perf(scan): ...`).
- **Sign-off.** Commits should include a `Signed-off-by:` trailer
  (`git commit -s`). The Lore convention also encourages capturing
  *why* the change exists, not just *what* changed.
- **No bundled fixes.** Don't roll style edits into behavior changes.
  Style-only changes are their own commit and PR.

## Code conventions

- **File size**: single-file limit is 800 lines (hard ceiling). When
  a module approaches that, split it into a directory module —
  `src/rules/` is the existing example. The split should be a separate
  pure-rename PR before any feature work lands on top.
- **No silent degradation.** Errors that cause user-visible wrong
  output must be raised, not logged at `warning` and replaced with a
  fallback. Optional features that can degrade safely (e.g. cache
  write failure) are the exception and must mark the degraded path
  explicitly.
- **No new aliases.** When renaming a function, type, or flag, remove
  every old reference in the same change — don't leave deprecated
  aliases.
- **Tests with every fix.** Every bug fix must add a regression test
  that fails before the fix and passes after. Refactors must keep
  existing tests green.
- **Symlink behavior** is part of the contract: symlinked candidates
  resolve to `Safety::Blocked` in both `scan` and `explain`. Anything
  touching the walker or `apply_path_safety` needs an explicit
  symlink case in tests.

## Adding a new ecosystem rule

Built-in rules live in `src/rules/`, one file per ecosystem. The
contract is `classify_candidate(parent, name, path) -> Option<CandidateDraft>`:

1. Pick the right file (`src/rules/node.rs`, `python.rs`, ...). If
   the ecosystem is new, add a sibling file and register it in
   `src/rules/mod.rs` and `src/rules/catalog.rs`.
2. Add at least one positive test in `tests/rules.rs` and one negative
   test that confirms the rule does *not* fire without its marker.
3. Generic directory names (`build`, `dist`, `out`, `target`,
   `vendor`) require a project marker — never classify them by name
   alone.
4. Update the **Supported Ecosystems** table in `README.md`.

User-extensible rules live in `.rclean.toml` (per scan root). User
rules layer *after* built-in rules and cannot produce `Safety::Blocked`.
The file format is documented in the README.

## Reporting bugs

- Functional bugs (wrong candidate, wrong size, wrong category):
  open a GitHub issue with a minimal reproducible directory layout
  (`mkdir -p` script is ideal).
- Security or trust-model issues: follow the private disclosure
  process in [`SECURITY.md`](SECURITY.md) — do not file a public issue.

## Release process

Versioning is `0.1.x` for the v0.1 series; behavior-breaking changes
require either a minor bump (post-1.0) or an explicit migration note
in `CHANGELOG.md`. The roadmap for the current minor series lives in
`docs/specs/v0.1.x-roadmap.md`.
