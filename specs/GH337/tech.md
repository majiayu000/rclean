# GH337 Technical Spec

## Linked Artifacts

- Issue: `#337`
- Product: `specs/GH337/product.md`
- Tasks: `specs/GH337/tasks.md`

## Baseline Evidence

At `origin/main` commit `d1b98d3ac8e2cdb8d0ad9bc1194050d391964258`:

- `src/scan/project.rs` is 436 lines.
- Lines 1-308, including the existing `#[cfg(test)]`, have SHA-256
  `bc2b6bc99e2557b5d8f01a283afff5d4cd18fbb6eef79a951913c356f11139f8`.
- The wrapper starts with `mod tests {` at line 309 and closes at line 436. Its body is lines 310-435, 126 lines,
  with raw SHA-256 `4ba139ab4649491e33c47c4d5ecaf80686114f2bdbb562aec6e59b7ce7384319`.
- Exactly 106 body lines are nonblank and start with the module's four-space nesting indent. Removing one such
  indent gives SHA-256 `a5f016f906edf17b19712b8517100ad6acc439f48e795098b381939f7b43b422`.
- Streaming that dedented body through `rustfmt --emit stdout --edition 2024` produces 125 lines with SHA-256
  `de0b441cca324c8eb81bf41e05a31fd52039bc3dbfc00f9e005e9b7c5bb32cf5`.
- The sole rustfmt layout change is that the cfg-Windows `symlink_file(...).unwrap()` call fits on one line after
  dedenting. Re-indent the formatted child into the wrapper and rustfmt the whole file, and the 436-line baseline
  reconstructs exactly.
- Five focused tests cover risk-cache reuse plus empty/single, ordering, traversal-boundary, and missing-project
  activity behavior. Fresh stable and exact Rust 1.95.0 runs pass 5/5.
- The body has no `file!`, `line!`, `column!`, `module_path!`, `include!`, or `#[path]` location-sensitive use.
- Search found GH284/#286 for activity precomputation and GH296/#300 for risk reuse, but no test extraction;
  GH314 explicitly excluded `scan/project.rs` from its separate project-rule test extraction.

## Design

### Parent module

Preserve lines 1-308 exactly and replace the inline wrapper/body with:

```rust
mod tests;
```

Because `#[cfg(test)]` is already line 308, the parent becomes 309 lines. Do not change production imports,
functions, types, visibility, cfg predicates, comments, or whitespace.

### Child test module

Add `src/scan/project/tests.rs` from baseline lines 310-435 by:

1. removing one four-space nesting indent from each nonblank line; then
2. applying `rustfmt --emit stdout --edition 2024`.

The expected child is 125 lines with the fixed formatted hash. Preserve the module namespace, all five test names,
`write_with_modified`, imports, item order, fixture paths, depth values, timestamps, cfg branches, symlink calls,
comments, and assertions. Do not add a nested wrapper, visibility, imports, helpers, lint allowances, skip markers,
or behavior changes.

## Exact Scope And Source Proof

Implementation verification must show:

```sh
git diff --name-only origin/main...HEAD
git diff --check origin/main...HEAD
wc -l src/scan/project.rs src/scan/project/tests.rs
test "$(sed -n '309p' src/scan/project.rs)" = 'mod tests;'
test -z "$(rg -n '^\s*pub(\(| )' src/scan/project/tests.rs || true)"
git diff origin/main...HEAD -- Cargo.toml Cargo.lock .github README.md docs specs
```

Expected contract:

- changed paths are exactly `src/scan/project.rs` and `src/scan/project/tests.rs`;
- parent/child line counts are exactly 309/125;
- the child adds no public or crate-public item;
- dependencies, workflows, README, docs, specs, and every other source/test file have empty diffs.

Run this fixed forward and whole-file rollback proof:

```sh
set -e
base=d1b98d3ac8e2cdb8d0ad9bc1194050d391964258

git show "$base":src/scan/project.rs | sed -n '1,308p' > /tmp/gh337-prefix.before
sed -n '1,308p' src/scan/project.rs > /tmp/gh337-prefix.after
test "$(shasum -a 256 /tmp/gh337-prefix.after | cut -d ' ' -f 1)" \
  = bc2b6bc99e2557b5d8f01a283afff5d4cd18fbb6eef79a951913c356f11139f8
diff -u /tmp/gh337-prefix.before /tmp/gh337-prefix.after

git show "$base":src/scan/project.rs | sed -n '310,435p' > /tmp/gh337-tests.raw
test "$(wc -l < /tmp/gh337-tests.raw | tr -d ' ')" -eq 126
test "$(rg -c '^ {4}.*\S$' /tmp/gh337-tests.raw)" -eq 106
test "$(shasum -a 256 /tmp/gh337-tests.raw | cut -d ' ' -f 1)" \
  = 4ba139ab4649491e33c47c4d5ecaf80686114f2bdbb562aec6e59b7ce7384319
sed 's/^    //' /tmp/gh337-tests.raw > /tmp/gh337-tests.dedented
test "$(shasum -a 256 /tmp/gh337-tests.dedented | cut -d ' ' -f 1)" \
  = a5f016f906edf17b19712b8517100ad6acc439f48e795098b381939f7b43b422
rustfmt --emit stdout --edition 2024 < /tmp/gh337-tests.dedented > /tmp/gh337-tests.before
test "$(wc -l < /tmp/gh337-tests.before | tr -d ' ')" -eq 125
test "$(shasum -a 256 /tmp/gh337-tests.before | cut -d ' ' -f 1)" \
  = de0b441cca324c8eb81bf41e05a31fd52039bc3dbfc00f9e005e9b7c5bb32cf5
diff -u /tmp/gh337-tests.before src/scan/project/tests.rs

{
  sed -n '1,308p' src/scan/project.rs
  printf 'mod tests {\n'
  sed '/./s/^/    /' src/scan/project/tests.rs
  printf '}\n'
} > /tmp/gh337-project.reconstructed.rs
rustfmt --edition 2024 /tmp/gh337-project.reconstructed.rs
git show "$base":src/scan/project.rs > /tmp/gh337-project.before.rs
diff -u /tmp/gh337-project.before.rs /tmp/gh337-project.reconstructed.rs
```

All diffs must be empty. The only allowed forward normalization is the fixed dedent plus Rust 2024 rustfmt; the
whole-file rollback must reproduce every baseline byte.

## Behavioral Proof

Source inspection and tests must confirm:

- `scan::project::tests` remains the namespace with the same five test names exactly once;
- `write_with_modified`, imports, fixture paths, depth values, timestamps, cfg branches, symlink calls, comments,
  and assertions are preserved by the normalized proof;
- no skip/ignore marker, assertion weakening, lint suppression, visibility increase, or alternate helper appears;
- the 308-line parent prefix and every non-target file remain unchanged.

## Verification

```sh
cargo test scan::project::tests -- --nocapture
rustup run 1.95.0 cargo test scan::project::tests -- --nocapture
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

- Risk: scan/risk behavior changes. Mitigation: fixed 308-line prefix hash and empty full-prefix diff.
- Risk: rustfmt hides a test edit. Mitigation: fixed raw/dedented/formatted hashes, exact child diff, and whole-file
  rollback to every baseline byte.
- Risk: cfg-specific symlink fixture changes on one platform. Mitigation: normalized proof plus macOS/Linux/Windows
  CI; no production symlink code is touched.
- Risk: private access or namespace changes. Mitigation: private child declaration, retained `use super::*`, and
  exact test inventory.
- Risk: tests weaken. Mitigation: normalized textual equivalence and VibeGuard integrity/weakening gates.

## Rollback

Re-indent the child into `mod tests { ... }`, rustfmt the whole parent with edition 2024, and remove the child.
The fixed rollback proof reconstructs the 436-line baseline exactly; no data, schema, runtime state, or migration
is involved.

## Acceptance Mapping

| Product criterion | Technical coverage |
| --- | --- |
| B-001 | Fixed 308-line prefix hash/diff and parent declaration |
| B-002 | Raw/dedented/formatted hashes and exact child diff |
| B-003 | Test/helper/cfg inventory and focused cross-platform tests |
| B-004 | Exact two-path scope, visibility, and 309/125 line gates |
| B-005 | Protected-scope diff and byte-identical production prefix |
| B-006 | Full/VibeGuard/review/CI/SpecRail gates |
