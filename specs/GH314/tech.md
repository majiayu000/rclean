# Extract Project Rule Tests Into A Dedicated Module - Tech Spec

## Linked Artifacts

- GitHub issue: `#314`
- Product spec: `specs/GH314/product.md`
- Tasks: `specs/GH314/tasks.md`
- Route: `write_spec`

## Baseline Evidence

| Area | Fresh evidence on `9a79ade` | Decision |
| --- | --- | --- |
| `src/rules/project.rs` | 630 lines | Split test-only content, not classifier behavior. |
| Production prefix | lines 1–382 plus test cfg | Preserve lines 1–383 exactly. |
| Inline wrapper | line 383 `#[cfg(test)]`, line 384 `mod tests {`, line 630 closing brace | Keep cfg line; replace wrapper with `mod tests;`. |
| Inline body | lines 385–629, 245 lines | Dedent one level and normalize with repository rustfmt; final child remains 245 lines. |
| Test inventory | 9 tests, 1 alias, 2 common helpers, 2 cfg-gated symlink helpers | Preserve all names, bodies and cfg gates. |
| Position-sensitive constructs | none in inline body | Recheck after move; do not add any. |
| Existing pattern | dedicated child test modules in scan/doctor/clean | Follow established Rust layout. |
| Duplicate search | no issue/PR/spec for project-rule test extraction | Use #314. |

## Design

Create the standard Rust child-module layout:

```text
src/rules/project.rs
src/rules/project/tests.rs
```

Implementation is intentionally mechanical:

1. Preserve `src/rules/project.rs` lines 1–383 exactly.
2. Replace `mod tests { ... }` with `mod tests;`, making the parent exactly 384 lines.
3. Create `src/rules/project/tests.rs` from baseline lines 385–629, removing exactly one four-space
   indentation level.
4. Run fmt and compare the child byte-for-byte with the same dedented baseline streamed through
   `rustfmt --emit stdout --edition 2024`, matching the crate edition in `Cargo.toml`.

Rust resolves `mod tests;` declared from `rules/project.rs` to `rules/project/tests.rs`. Existing explicit
`use super::{...}` imports keep private access within the parent module; no visibility change is necessary.

## Product-to-Change Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001 | unchanged parent prefix + external module declaration | prefix diff, line count, exact tail |
| B-002 | normalized dedented inline body in child | streamed-rustfmt diff, line count |
| B-003 | no edits inside moved inventory | normalized exact diff + source inventory |
| B-004 | no classifier contract edits | normalized exact diff + nine focused tests + CI |
| B-005 | two-path refactor-only manifest | name-status and dependency/workflow guards |
| B-006 | focused/full/MSRV/CI/PR gates | fresh local and remote evidence |

## Planned Change Manifest

| Path | Change |
| --- | --- |
| `src/rules/project.rs` | Replace inline test wrapper/body with `mod tests;`; preserve prefix. |
| `src/rules/project/tests.rs` | Add rustfmt-normalized dedented former inline body. |

No other file is permitted in the implementation diff.

## Exact Relocation Proof

Run after implementation and formatting while `origin/main` remains the implementation base:

```sh
test "$(wc -l < src/rules/project.rs | tr -d ' ')" -eq 384
test "$(wc -l < src/rules/project/tests.rs | tr -d ' ')" -eq 245
diff -u \
  <(git show origin/main:src/rules/project.rs | sed -n '1,383p') \
  <(sed -n '1,383p' src/rules/project.rs)
test "$(sed -n '384p' src/rules/project.rs)" = 'mod tests;'
git show origin/main:src/rules/project.rs \
  | sed -n '385,629p' \
  | sed 's/^    //' \
  | rustfmt --emit stdout --edition 2024 \
  | diff -u - src/rules/project/tests.rs
```

Both diffs must be empty. The stream uses the same installed rustfmt and crate edition as `cargo fmt`, so any
formatting change is reproduced from the original body instead of hand-approved. If base layout changes, fetch
and re-evaluate rather than changing expected coordinates opportunistically.

## Risks And Mitigations

- **Wrong module resolution:** use the established `project/tests.rs` child layout and compile all features.
- **Private import breakage:** retain the explicit `use super::{...}` block; do not change visibility.
- **Mechanical move hides edits:** require exact prefix and normalized child diffs after fmt.
- **Dropped outer cfg:** preserve `#[cfg(test)]` on the parent declaration.
- **Position-sensitive behavior:** re-run source scan for file/line/module/include/path constructs.
- **Cross-platform helper drift:** exact child proof plus macOS/Ubuntu/Windows CI covers cfg-specific symlink code.
- **Test weakening:** run exact proof plus VibeGuard test-integrity/test-weakening guards.
- **Main drift:** require fresh `origin/main` equality before implementation and merge.

## Verification Plan

```sh
cargo fmt -- --check
cargo test rules::project::tests -- --nocapture
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

Re-inline the child under `#[cfg(test)] mod tests { ... }`, restore one indentation level, run rustfmt, remove the
child file, and require the result to equal the original baseline body. No runtime, data, dependency or schema
rollback exists.

## Human Gates

- Spec and implementation remain separate PRs.
- Implementation starts only after the Spec PR merges on latest `origin/main`.
- Any request to edit classifier/test behavior is out of #314 scope and requires separate review.
- Merge only after current-head CI, independent review, zero unresolved review threads, clean merge state, valid
  signatures, SpecRail gate and the user standing authorization; never force push.
