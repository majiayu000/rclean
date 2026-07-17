# Extract Sizer Tests Into A Dedicated Module - Tech Spec

## Linked Artifacts

- Issue: `#320`
- Product: `specs/GH320/product.md`
- Tasks: `specs/GH320/tasks.md`
- Route: `write_spec`

## Baseline Evidence

| Area | Evidence on `bbf059c` | Decision |
| --- | --- | --- |
| `src/scan/sizer.rs` | 681 lines | Split test-only content. |
| Production prefix | lines 1–475 plus cfg at 476 | Preserve lines 1–476 exactly. |
| Wrapper | line 477 `mod tests {`, line 681 closing brace | Replace with `mod tests;`. |
| Body | lines 478–680, 203 raw lines | Dedent + edition-2024 rustfmt; final 203. |
| Inventory | 9 tests, 1 helper, 2 Unix cfg attributes | Preserve exact body and cfg. |
| Position-sensitive constructs | none | Recheck after move. |
| Duplicate search | sizing/performance issues only; no extraction work | Use #320. |

## Design

Use the standard child layout:

```text
src/scan/sizer.rs
src/scan/sizer/tests.rs
```

Preserve parent lines 1–476, replace the inline wrapper/body with `mod tests;`, create child from base lines
478–680 after one-level dedent, run fmt, then compare against the same baseline streamed through edition-2024
rustfmt. Existing `use super::*` preserves private access without visibility changes.

## Product-to-Change Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001 | unchanged prefix + child declaration | prefix/tail/line proof |
| B-002 | normalized dedented child | streamed diff + line proof |
| B-003 | exact moved inventory | normalized diff + source inventory |
| B-004 | unchanged sizing contracts | exact diff + nine focused tests + CI |
| B-005 | two-path manifest | name-status and dependency guards |
| B-006 | full gates | fresh local/remote evidence |

## Planned Change Manifest

| Path | Change |
| --- | --- |
| `src/scan/sizer.rs` | Replace inline test wrapper/body with `mod tests;`; preserve prefix. |
| `src/scan/sizer/tests.rs` | Add edition-2024-normalized dedented former body. |

No other implementation path is permitted.

## Exact Forward Proof

```sh
test "$(wc -l < src/scan/sizer.rs | tr -d ' ')" -eq 477
test "$(wc -l < src/scan/sizer/tests.rs | tr -d ' ')" -eq 203
diff -u \
  <(git show origin/main:src/scan/sizer.rs | sed -n '1,476p') \
  <(sed -n '1,476p' src/scan/sizer.rs)
test "$(sed -n '477p' src/scan/sizer.rs)" = 'mod tests;'
git show origin/main:src/scan/sizer.rs \
  | sed -n '478,680p' \
  | sed 's/^    //' \
  | rustfmt --emit stdout --edition 2024 \
  | diff -u - src/scan/sizer/tests.rs
```

Both diffs must be empty after `cargo fmt`.

## Risks And Mitigations

- Module/private access: established child layout + all-feature builds; no visibility edits.
- Hidden edits: exact normalized proof and two-path manifest.
- Wrong edition: proof explicitly matches `Cargo.toml` edition 2024; run stable and MSRV forward/rollback.
- Unix cfg drift: exact body proof plus macOS/Ubuntu/Windows CI.
- Parallel/warning drift: exact body proof plus focused sizing tests and full CI.
- Position semantics: scan moved paths for file/line/module/include/path constructs.
- Test weakening: exact proof plus VibeGuard integrity/weakening guards.
- Main drift: fetch/equality checks before implementation and merge.

## Verification Plan

```sh
cargo fmt -- --check
cargo test scan::sizer::tests -- --nocapture
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
and require stable/MSRV output to equal baseline lines 476–681.

## Human Gates

- Spec and implementation stay separate.
- Implementation waits for Spec merge and latest main.
- Any sizing/test behavior edit is out of scope.
- Merge only after all fresh gates and standing authorization; never force push.
