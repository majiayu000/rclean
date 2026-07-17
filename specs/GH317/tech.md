# Extract Watch Tests Into A Dedicated Module - Tech Spec

## Linked Artifacts

- Issue: `#317`
- Product: `specs/GH317/product.md`
- Tasks: `specs/GH317/tasks.md`
- Route: `write_spec`

## Baseline Evidence

| Area | Evidence on `581b475` | Decision |
| --- | --- | --- |
| `src/watch/mod.rs` | 553 lines | Split test-only content. |
| Production prefix | lines 1–318 plus cfg at 319 | Preserve lines 1–319 exactly. |
| Wrapper | line 320 `mod tests {`, line 553 closing brace | Replace with `mod tests;`. |
| Body | lines 321–552, 232 raw lines | Dedent + edition-2024 rustfmt; final 229. |
| Inventory | 9 tests, 2 helpers, 1 Unix-only test cfg | Preserve exact body and cfg. |
| Position-sensitive constructs | none | Recheck after move. |
| Duplicate search | GH274/GH293 behavior only; no extraction work | Use #317. |

## Design

Use the standard child layout:

```text
src/watch/mod.rs
src/watch/tests.rs
```

Preserve parent lines 1–319, replace the inline wrapper/body with `mod tests;`, create child from base lines
321–552 after one-level dedent, run fmt, then compare against the same baseline streamed through edition-2024
rustfmt. Existing `use super::*` preserves private access without visibility changes.

## Product-to-Change Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001 | unchanged prefix + child declaration | prefix/tail/line proof |
| B-002 | normalized dedented child | streamed diff + line proof |
| B-003 | exact moved inventory | normalized diff + source inventory |
| B-004 | unchanged watch contracts | exact diff + nine focused tests + CI |
| B-005 | two-path manifest | name-status and dependency guards |
| B-006 | full gates | fresh local/remote evidence |

## Planned Change Manifest

| Path | Change |
| --- | --- |
| `src/watch/mod.rs` | Replace inline test wrapper/body with `mod tests;`; preserve prefix. |
| `src/watch/tests.rs` | Add edition-2024-normalized dedented former body. |

No other implementation path is permitted.

## Exact Forward Proof

```sh
test "$(wc -l < src/watch/mod.rs | tr -d ' ')" -eq 320
test "$(wc -l < src/watch/tests.rs | tr -d ' ')" -eq 229
diff -u \
  <(git show origin/main:src/watch/mod.rs | sed -n '1,319p') \
  <(sed -n '1,319p' src/watch/mod.rs)
test "$(sed -n '320p' src/watch/mod.rs)" = 'mod tests;'
git show origin/main:src/watch/mod.rs \
  | sed -n '321,552p' \
  | sed 's/^    //' \
  | rustfmt --emit stdout --edition 2024 \
  | diff -u - src/watch/tests.rs
```

Both diffs must be empty after `cargo fmt`.

## Risks And Mitigations

- Module/private access: established child layout + all-feature builds; no visibility edits.
- Hidden edits: exact normalized proof and two-path manifest.
- Wrong edition: proof explicitly matches `Cargo.toml` edition 2024; run stable and MSRV forward/rollback.
- Unix cfg drift: exact body proof plus macOS/Ubuntu/Windows CI.
- Position semantics: scan moved paths for file/line/module/include/path constructs.
- Test weakening: exact proof plus VibeGuard integrity/weakening guards.
- Main drift: fetch/equality checks before implementation and merge.

## Verification Plan

```sh
cargo fmt -- --check
cargo test watch::tests -- --nocapture
git diff --check
git diff --name-status origin/main...HEAD
# exact forward proof above plus stable/MSRV reverse re-inline proof
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

Run all eight Rust VibeGuard guards, test-integrity/test-weakening/dependency guards, 800-line ceiling, SpecRail
packet/required PR gates, signatures, independent review, reviewThreads and four-check current-head CI. The unwrap
baseline remains three observations; workspace consistency remains a single-crate skip.

## Rollback

Re-inline child under `#[cfg(test)] mod tests { ... }`, restore one indent, run edition-2024 rustfmt, remove child,
and require stable/MSRV output to equal baseline lines 319–553.

## Human Gates

- Spec and implementation stay separate.
- Implementation waits for Spec merge and latest main.
- Any watch/test behavior edit is out of scope.
- Merge only after all fresh gates and standing authorization; never force push.
