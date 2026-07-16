# GitCache Test Module Split - Tech Spec

## Linked Artifacts

- GitHub issue: `#262`
- Product spec: `specs/GH262/product.md`
- Tasks: `specs/GH262/tasks.md`
- Route: `write_spec`

## Codebase Context

| Area | Current evidence | Decision |
| --- | --- | --- |
| `src/scan/git_cache.rs` | 743 lines; inline tests start after the production module body | Replace inline block with an external child-module declaration. |
| inline `mod tests` | 12 tests plus private helpers and platform-specific `slow_command` | Move the body verbatim to `src/scan/git_cache/tests.rs`. |
| Rust privacy | child modules can access private parent items | Keep `use super::*`; do not widen visibility. |

## Proposed Design

At the end of `src/scan/git_cache.rs`, use Rust's standard child-module layout:

```rust
#[cfg(test)]
mod tests;
```

Create `src/scan/git_cache/tests.rs` containing exactly the former `mod tests { ... }` body,
without the outer wrapper. Module identity therefore remains `scan::git_cache::tests`, so test
filters and names remain stable. Because it is a child of `git_cache`, `use super::*` retains
private access without changing production visibility.

## Product-to-Test Mapping

| Invariant | Evidence |
| --- | --- |
| B-001 production unchanged | compare pre/post parent content before the old test block; diff should only replace module body with declaration |
| B-002 test identity | sorted before/after `cargo test scan::git_cache::tests -- --list` names are equal and count is 12 |
| B-003 privacy unchanged | no `pub`/`pub(crate)` production diff; external child tests compile |
| B-004 test bodies unchanged | mechanical body extraction plus focused test pass |
| B-005 line limits | `wc -l src/scan/git_cache.rs src/scan/git_cache/tests.rs` |
| B-006 scope | `git diff --name-only origin/main...HEAD` equals the two-path manifest |
| B-007 gates | full local/MSRV/VibeGuard and current-head CI evidence |

## Planned Changes Manifest

| Path | Change |
| --- | --- |
| `src/scan/git_cache.rs` | Replace the inline test module with `#[cfg(test)] mod tests;`; no production edit. |
| `src/scan/git_cache/tests.rs` | Add the exact former inline test-module body. |

No other implementation path is permitted.

## Verification Plan

Focused and structural:

```sh
cargo test scan::git_cache::tests -- --list
cargo test scan::git_cache::tests
wc -l src/scan/git_cache.rs src/scan/git_cache/tests.rs
git diff --check
git diff --name-only origin/main...HEAD
```

Repository gates:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

Run all installed VibeGuard Rust guards. The remote CI gate must pass on Ubuntu, macOS, Windows,
and MSRV for the current head SHA.

## Risks And Mitigations

- **Module path drift:** assert the full sorted test-name set before and after.
- **Dropped test/helper during move:** move the complete body and compare mechanically; focused
  compile/test catches unresolved private helpers.
- **Visibility widening:** reject any production `pub` change.
- **Cross-platform helper loss:** retain both `#[cfg(unix)]` and `#[cfg(windows)]` functions.
- **Behavioral scope creep:** manifest is exactly two files and no production token change.

## Rollback

Revert the implementation commit to restore the inline module. There is no schema, dependency,
persistence, or runtime migration.

## Human Gates

- Spec and implementation remain separate PRs.
- Merge only after current-head CI, unresolved-thread, merge-state, scope and test-identity evidence
  are green.
- The user has provided standing merge authorization for this optimization run; never force push.
