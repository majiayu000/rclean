# GH326 Technical Spec

## Linked Artifacts

- Issue: `#326`
- Product: `specs/GH326/product.md`
- Tasks: `specs/GH326/tasks.md`

## Baseline Evidence

At `origin/main` commit `3850d8dc3c895cd5df18ae4599e47ef78a341e36`:

- `src/free.rs` is 480 lines and contains a 100-line inline test module.
- `src/output.rs` is 609 lines and contains an 84-line inline test module.
- `src/free.rs:402-436` and `src/output.rs:547-581` are byte-identical 35-line `report_with` functions with SHA-256
  `2bca0ca42f30ede77c24177af90f37cd8f2419da921490a80f0af7b582ada326`.
- Both files contain a 16-line candidate constructor with the same fields. The only semantic difference is that
  `free` accepts `Safety` while `output` hard-codes `Safety::Safe`.
- Search found no existing `test_support` module or exact issue/PR/spec for this duplication.
- Fresh focused baselines pass: three `free::tests` and the three `output::tests` named below.

## Design

### Test-only module boundary

Add `src/test_support.rs` and declare it in `src/main.rs` exactly as:

```rust
#[cfg(test)]
mod test_support;
```

The module is crate-private and exists only when Rust enables `cfg(test)`. It must not be public or feature-gated,
and production modules must not reference it outside their own `#[cfg(test)]` test modules.

### Shared typed constructors

`src/test_support.rs` owns exactly two crate-private functions:

```rust
pub(crate) fn ranking_candidate(
    name: &str,
    bytes: u64,
    safety: Safety,
    staleness_days: Option<u64>,
) -> Candidate

pub(crate) fn ranking_report(candidates: Vec<Candidate>) -> ScanReport
```

`ranking_candidate` preserves every current candidate field value and makes `Safety` explicit for both consumers.
`ranking_report` preserves the byte-identical report, summary, project, marker, activity, and byte-total values.
There is no generic builder, trait, macro, alias, wrapper, or configuration type.

### Consumer changes

Within `free.rs`'s existing `#[cfg(test)] mod tests`:

- import `ranking_candidate` and `ranking_report` from `crate::test_support`;
- remove the local candidate/report functions and their test-only model imports;
- replace calls directly, retaining the existing explicit safety arguments.

Within `output.rs`'s existing `#[cfg(test)] mod tests`:

- import the same two functions;
- remove the local candidate/report functions and their test-only model imports;
- replace calls directly and add `Safety::Safe` at every ranking-candidate call site.

Do not rename the shared functions at import sites. Do not move tests or edit assertions.

## Exact Test Inventory

The following six tests must remain present exactly once:

1. `free::tests::prefers_stale_candidates_over_larger_fresh_ones`
2. `free::tests::never_selects_non_safe_candidates_even_when_target_unmet`
3. `free::tests::prunes_picks_the_target_can_spare`
4. `output::tests::biggest_wins_ranks_stale_candidates_before_larger_fresh_ones`
5. `output::tests::biggest_wins_falls_back_to_size_within_the_same_staleness_group`
6. `output::tests::format_staleness_renders_days_or_dash`

The `clean::output::tests` namespace is unrelated and must not be counted as an `output::tests` ranking test.

## Scope Proof

Implementation verification must show:

```sh
git diff --name-only origin/main...HEAD
git diff --check origin/main...HEAD
rg -n 'fn (candidate|report_with)\(' src/free.rs src/output.rs
rg -n 'pub\(crate\) fn ranking_(candidate|report)\(' src/test_support.rs
rg -n 'mod test_support;' src/main.rs
wc -l src/main.rs src/free.rs src/output.rs src/test_support.rs
git diff -- Cargo.toml Cargo.lock .github
```

Expected source contract:

- changed paths are exactly the four paths defined by B-006;
- local helper search returns no matches;
- the shared helper search returns exactly two matches;
- the module declaration appears exactly once and is immediately gated by `#[cfg(test)]`;
- dependency/workflow diff is empty;
- all files remain below 800 lines.

The production prefixes before the existing inline test modules must remain byte-identical to the implementation
base:

- `src/free.rs` lines 1-379 at the base;
- `src/output.rs` lines 1-524 at the base.

`src/main.rs` may change only by the two-line gated module declaration. `src/test_support.rs` may contain only the
imports and two fixture constructors described above.

## Semantic Proof

Source inspection must confirm:

- all nine `free` candidate calls preserve their previous name, bytes, safety, and staleness arguments;
- all four `output` candidate calls preserve name, bytes, and staleness and now explicitly pass `Safety::Safe`;
- the three `free` and two output ranking tests call `ranking_report` once each;
- all six test names and every assertion line are unchanged;
- the shared report fields equal the baseline constructor exactly.

The implementation PR must include an inverse or normalized source proof showing that inlining the two shared
constructors would recover the baseline fixture values without drift.

## Verification

Focused stable verification:

```sh
cargo test free::tests -- --nocapture
cargo test output::tests -- --nocapture
```

Focused Rust 1.95.0 verification:

```sh
rustup run 1.95.0 cargo test free::tests -- --nocapture
rustup run 1.95.0 cargo test output::tests -- --nocapture
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

Run all eight Rust VibeGuard guards plus test-integrity, test-weakening, dependency, file-scope, signature,
reviewThreads, current-head CI, independent review, and SpecRail required PR gates.

## Risks And Mitigations

- Risk: output tests accidentally inherit non-safe state. Mitigation: every output call explicitly passes
  `Safety::Safe`.
- Risk: centralizing a fixture changes schema values. Mitigation: exact field inventory plus normalized/inverse
  source proof and unchanged assertions.
- Risk: test helpers leak into production. Mitigation: immediate `#[cfg(test)]` module gate and release/MSRV builds.
- Risk: a broad helper becomes an informal production API. Mitigation: crate-private, ranking-specific names and no
  production consumers.

## Rollback

Revert the implementation commit. Both test modules regain their local helpers; no data, schema, migration,
runtime state, or user-visible compatibility handling is required.

## Acceptance Mapping

| Product criterion | Technical coverage |
| --- | --- |
| B-001 | Test-only module boundary; shared typed constructors |
| B-002 | Free consumer changes; semantic proof |
| B-003 | Output consumer changes; semantic proof |
| B-004 | Scope proof; no aliases/wrappers |
| B-005 | `cfg(test)` boundary; release/MSRV gates |
| B-006 | Exact four-path scope; line counts |
| B-007 | Focused/full/VibeGuard/SpecRail/CI/review gates |
