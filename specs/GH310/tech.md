# Extract Deletion Tests Into A Dedicated Module - Tech Spec

## Linked Artifacts

- GitHub issue: `#310`
- Product spec: `specs/GH310/product.md`
- Tasks: `specs/GH310/tasks.md`
- Route: `write_spec`

## Baseline Evidence

| Area | Fresh evidence on `9e27d30` | Decision |
| --- | --- | --- |
| `src/clean/deletion.rs` | 546 lines | Split test-only content, not production behavior. |
| Production prefix | lines 1–305 plus blank line | Preserve lines 1–307 exactly. |
| Inline wrapper | line 307 `#[cfg(test)]`, line 308 `mod tests {`, line 546 closing brace | Keep cfg line, replace wrapper with `mod tests;`. |
| Inline module body | lines 309–545, 237 lines | Dedent exactly four spaces into child `tests.rs`. |
| Existing repository pattern | `scan/git_cache/tests.rs`, `doctor/tests.rs`, `clean/tests.rs` | Follow existing file/submodule layout. |
| Duplicate search | #112/#126 split top-level clean modules; no deletion-test extraction issue/PR/spec | Use new #310 rather than reopen unrelated scope. |

## Design

Create the standard Rust child-module layout:

```text
src/clean/deletion.rs
src/clean/deletion/tests.rs
```

Implementation is intentionally mechanical:

1. Preserve `src/clean/deletion.rs` lines 1–307 exactly.
2. Replace `mod tests { ... }` with `mod tests;` so the parent becomes exactly 308 lines.
3. Create `src/clean/deletion/tests.rs` from the old inline body, lines 309–545, removing exactly one
   four-space indentation level.
4. Run fmt, then require the extraction comparison to remain byte-for-byte equal after the planned dedent.

Rust resolves `mod tests;` declared from `clean/deletion.rs` to `clean/deletion/tests.rs`. Inside the child,
`use super::*` still imports private items from `clean::deletion`, so no visibility or production code change
is necessary.

## Product-to-Change Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001 | unchanged parent prefix + external module declaration | prefix diff, line count, exact tail |
| B-002 | exact dedented inline body in child file | process-substitution `diff -u`, line count |
| B-003 | no edits inside moved body | exact relocation diff + focused tests/source checks |
| B-004 | no fixture/cfg/predicate edits | exact relocation diff + three-platform CI |
| B-005 | two-path refactor-only manifest | name-status and dependency/workflow guards |
| B-006 | focused/full/MSRV/CI/PR gates | fresh local and remote evidence |

## Planned Change Manifest

| Path | Change |
| --- | --- |
| `src/clean/deletion.rs` | Replace the inline test wrapper/body with `mod tests;`; preserve prefix. |
| `src/clean/deletion/tests.rs` | Add exact dedented copy of the former inline module body. |

No other file is permitted in the implementation diff.

## Exact Relocation Proof

Run after implementation and formatting, while the implementation branch still has the Spec-merged
`origin/main` as its base:

```sh
test "$(wc -l < src/clean/deletion.rs | tr -d ' ')" -eq 308
test "$(wc -l < src/clean/deletion/tests.rs | tr -d ' ')" -eq 237
diff -u \
  <(git show origin/main:src/clean/deletion.rs | sed -n '1,307p') \
  <(sed -n '1,307p' src/clean/deletion.rs)
test "$(sed -n '308p' src/clean/deletion.rs)" = 'mod tests;'
diff -u \
  <(git show origin/main:src/clean/deletion.rs | sed -n '309,545p' | sed 's/^    //') \
  src/clean/deletion/tests.rs
```

An empty `diff -u` result is required. Do not update the expected line numbers or normalize any other text if
the base changes; fetch and re-evaluate the baseline instead.

## Risks And Mitigations

- **Wrong module resolution:** use the repository-established `deletion/tests.rs` layout and compile all
  feature combinations.
- **Private helper access breaks:** retain `use super::*`; do not change visibility.
- **Mechanical move hides edits:** exact prefix and dedented-body diffs must be empty after fmt.
- **Dropped outer cfg:** preserve `#[cfg(test)]` on the parent `mod tests;` declaration.
- **Test weakening:** preserve exact body and run VibeGuard test-integrity/test-weakening guards.
- **Production behavior drift:** parent production prefix must be identical and no production file other than
  the wrapper replacement is changed.
- **Main drift:** require fresh `origin/main` equality before implementation and merge; stop if line baseline
  changes.

## Verification Plan

```sh
cargo fmt -- --check
cargo test clean::deletion::tests -- --nocapture
git diff --check
git diff --name-status origin/main...HEAD
# exact relocation proof above
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

Run all eight installed Rust VibeGuard guards plus test-integrity, test-weakening and dependency guards. Baseline
unwrap remains exactly three existing observations; workspace consistency remains an expected single-crate skip.
Run SpecRail packet check and required PR gate, then require current-head Ubuntu/macOS/Windows/MSRV CI.

## Rollback

Re-inline the exact contents of `src/clean/deletion/tests.rs` under `#[cfg(test)] mod tests { ... }` and remove
the child file. No runtime, data, dependency or schema rollback exists.

## Human Gates

- Spec and implementation remain separate PRs.
- Implementation starts only after the Spec PR merges on latest `origin/main`.
- Any request to edit test behavior or production deletion code is out of #310 scope and must stop for separate
  review.
- Merge only after current-head CI, independent review, zero unresolved review threads, clean merge state, valid
  signatures, SpecRail gate and the user standing authorization; never force push.
