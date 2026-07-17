# GH334 Technical Spec

## Linked Artifacts

- Issue: `#334`
- Product: `specs/GH334/product.md`
- Tasks: `specs/GH334/tasks.md`

## Baseline Evidence

At `origin/main` commit `7ad8d2de97bfb85b78f5b879d38e57a3725c1fe6`:

- `src/free.rs` is 427 lines.
- The production prefix is lines 1-379 with SHA-256
  `dd0931b99129d33939c801710f0093a7fc477dd7dddb3f29b52da6a2874b47de`.
- The inline wrapper is lines 380-427; its body is lines 382-426, 45 lines, with raw SHA-256
  `67a4e2cd5036c14370f77d114615cece801dcf53182744bd99239ad409202a40`.
- Exactly 42 body lines are nonblank and start with the module's four-space nesting indent. Removing exactly one
  such indent with `sed 's/^    //'` produces SHA-256
  `6b701734a5adb3dc60eff79f4c9566e6bf4f5c50fe4c1f5440e40fefd6f75bc6` and passes
  `rustfmt --edition 2024 --check`.
- The test names are `prefers_stale_candidates_over_larger_fresh_ones`,
  `never_selects_non_safe_candidates_even_when_target_unmet`, and `prunes_picks_the_target_can_spare`.
- `src/test_support.rs` has SHA-256
  `4537ce5bda702273d89a4583f9b4f36a09d865c259fdcfa0d66ca509f8d5b55d`.
- Fresh stable and exact Rust 1.95.0 `cargo test free::tests -- --nocapture` each run exactly three passing tests.
- Search across issues, PRs, `docs/specs/`, and `specs/` found GH326/#328 for ranking fixture deduplication, but no
  existing work for externalizing the `free` test module.

## Design

### Parent module

Keep `src/free.rs:1-379` byte-identical and replace the inline test module with exactly:

```rust
#[cfg(test)]
mod tests;
```

The expected parent is 381 lines. Do not change imports, functions, types, visibility, cfg predicates, comments,
or production whitespace.

### Child test module

Add `src/free/tests.rs` containing the normalized form of baseline lines 382-426. The only normalization is
removing one four-space nesting indent from every nonblank line. Preserve blank lines, item order, `use super::*`,
the `crate::test_support` import, test attributes, names, fixture calls, inputs, comments, and assertions.

The file is a private `#[cfg(test)]` child selected by the parent declaration. Do not add a nested wrapper, public
visibility, helpers, aliases, imports, assertions, lint allowances, or test behavior.

## Exact Scope And Source Proof

Implementation verification must show:

```sh
git diff --name-only origin/main...HEAD
git diff --check origin/main...HEAD
wc -l src/free.rs src/free/tests.rs
test "$(rg -n '^#\[cfg\(test\)\]$' src/free.rs | wc -l | tr -d ' ')" -eq 1
test "$(rg -n '^mod tests;$' src/free.rs | wc -l | tr -d ' ')" -eq 1
test -z "$(rg -n '^\s*pub(\(| )' src/free/tests.rs || true)"
git diff origin/main...HEAD -- src/test_support.rs Cargo.toml Cargo.lock .github README.md docs
```

Expected contract:

- changed paths are exactly `src/free.rs` and `src/free/tests.rs`;
- parent/child line counts are 381/45, both below 400;
- the child adds no public or crate-public item;
- test support, dependencies, workflows, README, and docs have empty diffs.

Run this fixed reconstruction proof from the implementation worktree:

```sh
set -e
base=7ad8d2de97bfb85b78f5b879d38e57a3725c1fe6

git show "$base":src/free.rs | sed -n '1,379p' > /tmp/gh334-prefix.before
sed -n '1,379p' src/free.rs > /tmp/gh334-prefix.after
test "$(shasum -a 256 /tmp/gh334-prefix.after | cut -d ' ' -f 1)" \
  = dd0931b99129d33939c801710f0093a7fc477dd7dddb3f29b52da6a2874b47de
diff -u /tmp/gh334-prefix.before /tmp/gh334-prefix.after

git show "$base":src/free.rs | sed -n '382,426p' > /tmp/gh334-tests.raw
test "$(wc -l < /tmp/gh334-tests.raw | tr -d ' ')" -eq 45
test "$(rg -c '^ {4}.*\S$' /tmp/gh334-tests.raw)" -eq 42
test "$(shasum -a 256 /tmp/gh334-tests.raw | cut -d ' ' -f 1)" \
  = 67a4e2cd5036c14370f77d114615cece801dcf53182744bd99239ad409202a40
sed 's/^    //' /tmp/gh334-tests.raw > /tmp/gh334-tests.before
test "$(shasum -a 256 /tmp/gh334-tests.before | cut -d ' ' -f 1)" \
  = 6b701734a5adb3dc60eff79f4c9566e6bf4f5c50fe4c1f5440e40fefd6f75bc6
diff -u /tmp/gh334-tests.before src/free/tests.rs

git show "$base":src/free.rs | sed -n '380,427p' > /tmp/gh334-wrapper.before
{
  printf '#[cfg(test)]\nmod tests {\n'
  sed '/./s/^/    /' src/free/tests.rs
  printf '}\n'
} > /tmp/gh334-wrapper.after
diff -u /tmp/gh334-wrapper.before /tmp/gh334-wrapper.after

test "$(shasum -a 256 src/test_support.rs | cut -d ' ' -f 1)" \
  = 4537ce5bda702273d89a4583f9b4f36a09d865c259fdcfa0d66ca509f8d5b55d
```

Every `diff -u` must exit zero. The proof may remove only the fixed module-nesting indent for forward extraction
and restore it only on nonblank lines for rollback reconstruction.

## Behavioral Proof

Source inspection and test output must confirm:

- `free::tests` remains the namespace and contains exactly the same three names once each;
- all fixture calls, safety variants, numeric/staleness inputs, comments, and assertions are unchanged;
- no production line, test-support helper, public surface, feature gate, or dependency changes;
- no test skip/ignore marker, weakened assertion, lint suppression, or alternate helper is introduced.

## Verification

Focused stable verification:

```sh
cargo test free::tests -- --nocapture
```

Focused exact MSRV verification:

```sh
rustup run 1.95.0 cargo test free::tests -- --nocapture
```

Full gate:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

Run all eight installed Rust VibeGuard guards plus test-integrity, test-weakening, dependency-change, source,
scope, signature, independent-review, current-head CI, reviewThreads, and SpecRail required PR gates.

## Risks And Mitigations

- Risk: production behavior changes while shortening the parent. Mitigation: fixed production-prefix SHA and empty
  full-prefix diff.
- Risk: dedent or formatting changes a test. Mitigation: counted raw indentation, fixed normalized SHA, exact child
  diff, and exact rollback reconstruction.
- Risk: tests lose private access or change namespace. Mitigation: private child declaration, retained
  `use super::*`, exact test-name inventory, and focused tests.
- Risk: GH326 fixture support drifts. Mitigation: fixed `src/test_support.rs` hash and empty protected-scope diff.
- Risk: assertions are weakened. Mitigation: exact body reconstruction and VibeGuard test-integrity/weakening gates.

## Rollback

Re-indent the child body into the original inline wrapper and remove `src/free/tests.rs`; the fixed rollback proof
must reconstruct baseline lines 380-427 exactly. No data, schema, migration, runtime state, or compatibility work
is involved.

## Acceptance Mapping

| Product criterion | Technical coverage |
| --- | --- |
| B-001 | Fixed production-prefix hash and parent declaration |
| B-002 | Raw/normalized hashes and exact forward/rollback proof |
| B-003 | Test inventory and focused stable/MSRV tests |
| B-004 | Exact two-path scope, visibility, and line-count gates |
| B-005 | Test-support hash and protected-scope diff |
| B-006 | Full/VibeGuard/review/CI/SpecRail gates |
