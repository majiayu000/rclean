# GH329 Technical Spec

## Linked Artifacts

- Issue: `#329`
- Product: `specs/GH329/product.md`
- Tasks: `specs/GH329/tasks.md`

## Baseline Evidence

At `origin/main` commit `1bc358d4589233b6cf7d7c70d99a051fe758c2c1`:

- `src/doctor.rs` is 653 lines; `diagnose_with_options` spans lines 64-577.
- The ordered common-entry body is lines 74-341 with SHA-256
  `fc50a7f098e576682b6e9d25fc6985d64e376724eb28643c67e459ea0a0c77e1`.
- That common body contains exactly 12 `&home` helper arguments. After the new child boundary changes `home` from
  an owned `PathBuf` local to an `&Path` parameter, those expressions must become `home` to avoid Clippy
  `needless_borrow`; the fixed normalized body SHA-256 is
  `83c6c74784439472928fa877f641afb219fed2eadaa219da717739022bf9f01f`.
- The ordered platform-entry body is lines 343-570 with SHA-256
  `427d6760ed94b231bfec163b6818b04b2d5bcf77bd4081e12b2eba68fc86c98e`.
- The low-level helper/test-module tail is lines 579-653 with SHA-256
  `1c8c15f05c3dbd10c62296e4a9271d8e5aaf7bd86ba5f8342d5b4ed07884bae0`.
- `src/doctor/tests.rs` is 106 lines with SHA-256
  `07636a0b03500112f943da9e73588f3dec31e3478337d687c7a8bfff3c4e49c8`.
- Fresh `cargo check --all-targets --all-features` and `cargo test doctor::tests -- --nocapture` pass; the latter
  runs exactly three passing doctor tests.
- Search found no production-constructor split. #112/#124-#126 cover other modules and #265/#267 cover only the
  doctor test extraction/catalog assertion.

## Design

### Common-entry module

Add `src/doctor/common_entries.rs` with one private-to-parent entry point:

```rust
pub(super) fn collect(home: &Path) -> Vec<DoctorEntry>
```

Its body is the existing `src/doctor.rs:74-341` sequence plus a final `entries` return, with exactly the 12
`&home` helper arguments replaced by `home`. This is required because the child parameter is already `&Path`; it
preserves the effective `&Path` passed to every helper while satisfying `clippy::needless_borrow`. It owns the
anchor helper imports currently used by that sequence. It must not reorder, consolidate, data-drive, or otherwise
rewrite the entry construction.

### Platform-entry module

Add `src/doctor/platform_entries.rs` with one private-to-parent entry point:

```rust
pub(super) fn extend(entries: &mut Vec<DoctorEntry>, home: &Path)
```

Its body is the exact existing `src/doctor.rs:343-570` sequence. Platform-only imports must retain cfg guards so
all platforms remain warning-free. It appends to the supplied vector and returns no replacement collection.

### Parent orchestration

`src/doctor.rs` declares both modules privately beside `anchors`. `diagnose_with_options` retains current HOME
resolution and empty-HOME return, then performs only this ordered orchestration:

1. `common_entries::collect(&home)`;
2. `platform_entries::extend(&mut entries, &home)`;
3. append `check_docker_daemon(Duration::from_secs(5))` only when `include_docker` is true;
4. return `DoctorReport { entries }`.

Keep `diagnose`, all report/options/status types, `check_anchor`, `skipped_anchor`, `check_any_anchor`,
`check_docker_daemon`, and `#[cfg(test)] mod tests;` in the parent. Existing public signatures and visibility remain
unchanged. Child functions are `pub(super)`, never `pub(crate)` or `pub`.

## Exact Scope And Source Proof

Implementation verification must show:

```sh
git diff --name-only origin/main...HEAD
git diff --check origin/main...HEAD
wc -l src/doctor.rs src/doctor/common_entries.rs src/doctor/platform_entries.rs
rg -x 'pub\(super\) fn collect\(home: &Path\) -> Vec<DoctorEntry> \{' src/doctor/common_entries.rs
rg -x 'pub\(super\) fn extend\(entries: &mut Vec<DoctorEntry>, home: &Path\) \{' src/doctor/platform_entries.rs
test "$(rg -n '^\s*pub(\(| )' src/doctor/common_entries.rs src/doctor/platform_entries.rs | wc -l | tr -d ' ')" -eq 2
git show 1bc358d4589233b6cf7d7c70d99a051fe758c2c1:src/doctor.rs | rg '^\s*pub(\(| )' > /tmp/gh329-parent-public.before
rg '^\s*pub(\(| )' src/doctor.rs > /tmp/gh329-parent-public.after
diff -u /tmp/gh329-parent-public.before /tmp/gh329-parent-public.after
git diff origin/main...HEAD -- src/doctor/tests.rs src/doctor/anchors.rs Cargo.toml Cargo.lock .github README.md
```

Expected contract:

- changed paths are exactly the three B-006 paths;
- the two exact child entry-point signatures each occur once and the exhaustive child `pub*` count is exactly two;
- the parent public surface, including functions, types, fields, constants, statics, aliases, and traits, has an
  empty baseline/head diff;
- tests, anchors, dependencies, workflows, and README diff are empty;
- all three affected production files are below 400 lines.

Run the following fixed reconstruction proof from the implementation worktree. Each `diff -u` must exit zero. The
only content normalization allowed is the exactly counted 12-token `&home` to `home` replacement in the common
baseline; the commands may otherwise remove only child-function wrapper lines and the common module's final
`entries` return expression:

```sh
set -e
base=1bc358d4589233b6cf7d7c70d99a051fe758c2c1

git show "$base":src/doctor.rs | sed -n '74,341p' > /tmp/gh329-common.raw
test "$(rg -o '&home' /tmp/gh329-common.raw | wc -l | tr -d ' ')" -eq 12
sed 's/&home/home/g' /tmp/gh329-common.raw > /tmp/gh329-common.before
test "$(shasum -a 256 /tmp/gh329-common.before | cut -d ' ' -f 1)" \
  = 83c6c74784439472928fa877f641afb219fed2eadaa219da717739022bf9f01f
sed -n '/^pub(super) fn collect/,/^}/p' src/doctor/common_entries.rs \
  | sed '1d;$d' | sed '$d' > /tmp/gh329-common.after
test -z "$(rg -o '&home' /tmp/gh329-common.after || true)"
diff -u /tmp/gh329-common.before /tmp/gh329-common.after

git show "$base":src/doctor.rs | sed -n '343,570p' > /tmp/gh329-platform.before
sed -n '/^pub(super) fn extend/,/^}/p' src/doctor/platform_entries.rs \
  | sed '1d;$d' > /tmp/gh329-platform.after
diff -u /tmp/gh329-platform.before /tmp/gh329-platform.after

git show "$base":src/doctor.rs | sed -n '579,653p' > /tmp/gh329-parent-tail.before
sed -n '/^fn check_anchor/,$p' src/doctor.rs > /tmp/gh329-parent-tail.after
diff -u /tmp/gh329-parent-tail.before /tmp/gh329-parent-tail.after

test "$(shasum -a 256 src/doctor/tests.rs | cut -d ' ' -f 1)" \
  = 07636a0b03500112f943da9e73588f3dec31e3478337d687c7a8bfff3c4e49c8
```

The implementation PR must record these fresh results rather than relying on visual similarity. If later merged
work changes any recorded baseline slice before implementation starts, update the Spec in a separate reviewed
Spec change instead of silently changing the comparison base or normalization.

## Behavioral Proof

Source inspection and CI must confirm:

- the common and platform sequences concatenate in their original order;
- all existing cfg predicates remain attached to the same entry blocks;
- all rule IDs, paths, reason strings, status variants, and effective anchor-helper arguments are unchanged; the
  only source spelling change inside moved bodies is the fixed 12-token common-module borrow simplification;
- missing HOME returns before child-module calls;
- Docker remains optional, last, and uses `Duration::from_secs(5)`;
- the three doctor test names and exact test file hash remain unchanged.

## Verification

Focused stable verification:

```sh
cargo test doctor::tests -- --nocapture
```

Focused exact MSRV verification:

```sh
rustup run 1.95.0 cargo test doctor::tests -- --nocapture
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

Run all eight installed Rust VibeGuard guards plus scope, test-integrity, source-equivalence, signature,
reviewThreads, current-head CI, independent review, and SpecRail required PR gates.

## Risks And Mitigations

- Risk: moving cfg blocks changes one platform's output. Mitigation: exact body reconstruction plus Ubuntu, macOS,
  Windows, and MSRV CI.
- Risk: entry order changes at the module join. Mitigation: common returns the existing prefix, platform appends the
  existing suffix, and Docker remains the final parent append.
- Risk: borrow normalization masks another edit. Mitigation: require exactly 12 baseline tokens, zero child
  `&home` tokens, the fixed normalized hash, and an empty full-body diff.
- Risk: private helpers become broader APIs. Mitigation: exactly two `pub(super)` functions and no re-exports.
- Risk: a cleanup registry is introduced during the split. Mitigation: mechanical move only; no tables, macros,
  aliases, traits, builders, or dependencies.

## Rollback

Revert the implementation commit. The two child bodies return to `diagnose_with_options`; no schema, migration,
runtime state, data, or compatibility handling is required.

## Acceptance Mapping

| Product criterion | Technical coverage |
| --- | --- |
| B-001 | Two private child entry points; no re-export |
| B-002 | Common exact extraction proof |
| B-003 | Platform exact extraction proof; cross-platform CI |
| B-004 | Parent orchestration and Docker-last contract |
| B-005 | Behavioral proof; no public/scope drift |
| B-006 | Exact three-path scope and line-count gate |
| B-007 | Unchanged test hash and focused stable/MSRV tests |
| B-008 | Full/VibeGuard/SpecRail/CI/review gates |
