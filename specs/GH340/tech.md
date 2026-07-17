# GH340 Technical Spec

## Linked Artifacts

- Issue: `#340`
- Product: `specs/GH340/product.md`
- Tasks: `specs/GH340/tasks.md`

## Baseline Evidence

At `origin/main` commit `3f06285869c9ac52f86ebaa90c563a95de5f1866`:

- `src/scan/walker.rs` is 458 lines.
- Lines 1-339, including the existing `#[cfg(test)]`, have SHA-256
  `62ab058a32e0ca66b6252d4c980af613165956cad472470f3b760f25c62575ef`.
- The wrapper starts with `mod tests {` at line 340 and closes at line 458. Its body is lines 341-457, 117 lines,
  with raw SHA-256 `4b51e5cdb889be2be385862eb0a3f6d308cfa19ab7fc363269008f2451920622`.
- Exactly 104 body lines are nonblank and start with the module's four-space nesting indent. Removing one such
  indent gives SHA-256 `55051f7b83ba00cc1d8e84c9e4506674213badc89de6d5ebaf727cbaf4f3808d`.
- Streaming that dedented body through `rustfmt --emit stdout --edition 2024` produces the same 117 lines and
  SHA-256 `55051f7b83ba00cc1d8e84c9e4506674213badc89de6d5ebaf727cbaf4f3808d`; Rustfmt changes no bytes.
- Re-indent the formatted child into the wrapper and Rustfmt the whole file, and the 458-line baseline
  reconstructs exactly.
- Four focused tests cover size/draft mutex poisoning through direct `into_inner` and `WalkLocal` drop paths.
  Fresh stable and exact Rust 1.95.0 runs pass 4/4.
- The body has no `file!`, `line!`, `column!`, `module_path!`, `include!`, or `#[path]` location-sensitive use.
- Search found #133/#146 for poisoned-lock behavior, #54 for the parallel walker, and #59 for the scan module split,
  but no walker test extraction in GitHub issues/PRs, `docs/specs/`, `specs/`, or file history.

## Design

### Parent module

Preserve lines 1-339 exactly and replace the inline wrapper/body with:

```rust
mod tests;
```

Because `#[cfg(test)]` is already line 339, the parent becomes 340 lines. Do not change production imports,
functions, types, visibility, cfg predicates, comments, or whitespace.

### Child test module

Add `src/scan/walker/tests.rs` from baseline lines 341-457 by:

1. removing one four-space nesting indent from each nonblank line; then
2. applying `rustfmt --emit stdout --edition 2024`.

The expected child is 117 lines with the fixed formatted hash. Preserve the module namespace, all four test names,
imports, item order, mutex targets, catch-unwind boundaries, panic/error strings, candidate draft fields, paths,
and assertions. Do not add a nested wrapper, visibility, imports, helpers, lint allowances, skip markers, or
behavior changes.

## Exact Scope And Source Proof

Implementation verification must show:

```sh
git diff --name-only origin/main...HEAD
git diff --check origin/main...HEAD
wc -l src/scan/walker.rs src/scan/walker/tests.rs
test "$(sed -n '340p' src/scan/walker.rs)" = 'mod tests;'
test -z "$(rg -n '^\s*pub(\(| )' src/scan/walker/tests.rs || true)"
git diff origin/main...HEAD -- Cargo.toml Cargo.lock .github README.md docs specs
```

Expected contract:

- changed paths are exactly `src/scan/walker.rs` and `src/scan/walker/tests.rs`;
- parent/child line counts are exactly 340/117;
- the child adds no public or crate-public item;
- dependencies, workflows, README, docs, specs, and every other source/test file have empty diffs.

Run this fixed forward and whole-file rollback proof:

```sh
set -e
base=3f06285869c9ac52f86ebaa90c563a95de5f1866

git show "$base":src/scan/walker.rs | sed -n '1,339p' > /tmp/gh340-prefix.before
sed -n '1,339p' src/scan/walker.rs > /tmp/gh340-prefix.after
test "$(shasum -a 256 /tmp/gh340-prefix.after | cut -d ' ' -f 1)" \
  = 62ab058a32e0ca66b6252d4c980af613165956cad472470f3b760f25c62575ef
diff -u /tmp/gh340-prefix.before /tmp/gh340-prefix.after

git show "$base":src/scan/walker.rs | sed -n '341,457p' > /tmp/gh340-tests.raw
test "$(wc -l < /tmp/gh340-tests.raw | tr -d ' ')" -eq 117
test "$(rg -c '^ {4}.*\S$' /tmp/gh340-tests.raw)" -eq 104
test "$(shasum -a 256 /tmp/gh340-tests.raw | cut -d ' ' -f 1)" \
  = 4b51e5cdb889be2be385862eb0a3f6d308cfa19ab7fc363269008f2451920622
sed 's/^    //' /tmp/gh340-tests.raw > /tmp/gh340-tests.dedented
test "$(shasum -a 256 /tmp/gh340-tests.dedented | cut -d ' ' -f 1)" \
  = 55051f7b83ba00cc1d8e84c9e4506674213badc89de6d5ebaf727cbaf4f3808d
rustfmt --emit stdout --edition 2024 < /tmp/gh340-tests.dedented > /tmp/gh340-tests.before
test "$(wc -l < /tmp/gh340-tests.before | tr -d ' ')" -eq 117
test "$(shasum -a 256 /tmp/gh340-tests.before | cut -d ' ' -f 1)" \
  = 55051f7b83ba00cc1d8e84c9e4506674213badc89de6d5ebaf727cbaf4f3808d
diff -u /tmp/gh340-tests.before src/scan/walker/tests.rs

{
  sed -n '1,339p' src/scan/walker.rs
  printf 'mod tests {\n'
  sed '/./s/^/    /' src/scan/walker/tests.rs
  printf '}\n'
} > /tmp/gh340-walker.reconstructed.rs
rustfmt --edition 2024 /tmp/gh340-walker.reconstructed.rs
git show "$base":src/scan/walker.rs > /tmp/gh340-walker.before.rs
diff -u /tmp/gh340-walker.before.rs /tmp/gh340-walker.reconstructed.rs
```

All diffs must be empty. The only allowed forward normalization is the fixed dedent plus Rust 2024 Rustfmt; the
whole-file rollback must reproduce every baseline byte.

## Behavioral Proof

Source inspection and tests must confirm:

- `scan::walker::tests` remains the namespace with the same four test names exactly once;
- imports, mutex targets, catch-unwind boundaries, panic/error strings, candidate draft fields, paths, and
  assertions are preserved by the normalized proof;
- no skip/ignore marker, assertion weakening, lint suppression, visibility increase, or alternate helper appears;
- the 339-line parent prefix and every non-target file remain unchanged.

## Verification

```sh
cargo test scan::walker::tests -- --nocapture
rustup run 1.95.0 cargo test scan::walker::tests -- --nocapture
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

Run all eight installed Rust VibeGuard guards plus test-integrity, test-weakening, dependency-change, exact source,
scope, signature, independent-review, current-head CI, reviewThreads, and SpecRail required PR gates.

## Risks And Mitigations

- Risk: walker or poison behavior changes. Mitigation: fixed 339-line prefix hash and empty full-prefix diff.
- Risk: Rustfmt hides a test edit. Mitigation: fixed raw/dedented/formatted hashes, exact child diff, and whole-file
  rollback to every baseline byte.
- Risk: panic/error contract changes. Mitigation: exact textual equivalence plus focused poison-path tests.
- Risk: private access or namespace changes. Mitigation: private child declaration, retained `use super::*`, and
  exact test inventory.
- Risk: tests weaken. Mitigation: normalized textual equivalence and VibeGuard integrity/weakening gates.

## Rollback

Re-indent the child into `mod tests { ... }`, Rustfmt the whole parent with edition 2024, and remove the child.
The fixed rollback proof reconstructs the 458-line baseline exactly; no data, schema, runtime state, or migration
is involved.

## Acceptance Mapping

| Product criterion | Technical coverage |
| --- | --- |
| B-001 | Fixed 339-line prefix hash/diff and parent declaration |
| B-002 | Raw/dedented/formatted hashes and exact child diff |
| B-003 | Test/import/fixture/panic/error/assertion inventory and focused tests |
| B-004 | Exact two-path scope, visibility, and 340/117 line gates |
| B-005 | Protected-scope diff and byte-identical production prefix |
| B-006 | Full/VibeGuard/review/CI/SpecRail gates |
