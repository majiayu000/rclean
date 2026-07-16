# Release Input Dry-Run Coverage - Tech Spec

## Linked Artifacts

- GitHub issue: `#271`
- Product spec: `specs/GH271/product.md`
- Tasks: `specs/GH271/tasks.md`
- Route: `write_spec`

## Codebase Context

| Area | Evidence | Decision |
| --- | --- | --- |
| PR path filter | only `release.yml`; PR #213 had no Release check | Add all direct release-input paths. |
| notes dry-run | hardcodes `0.2.0` | Read current package version through one helper. |
| notes extraction | same AWK exists in PR and tag jobs | Move to one fail-closed script. |
| GH268 scripts | verifier and test duplicate Cargo metadata selection | Extract one raw package-version helper. |
| release graph | current PR proves 11 SUCCESS + 2 expected SKIPPED | Preserve matrix, needs and event conditions. |

## Proposed Design

Add `.github/scripts/package-version.sh`:

- resolve repository root from `BASH_SOURCE` and run there;
- require `cargo` and `jq`;
- run `cargo metadata --no-deps --format-version 1`;
- require exactly one package named `rclean-cli`;
- print only its version to stdout; errors go to stderr and exit non-zero.

Refactor `.github/scripts/verify-release-version.sh` to call `package-version.sh`. Its one-tag
interface, prefix checks, exact `v<version>` comparison and messages remain behaviorally equivalent.

Add `.github/scripts/extract-release-notes.sh` with arguments `<version> <changelog> <output>`:

- reject missing/empty arguments;
- extract the exact heading with the existing boundary rule;
- write through a temporary sibling and move only after a non-empty result;
- remove the temporary file on every failure;
- report missing and empty sections distinctly.

Rename `.github/scripts/test-verify-release-version.sh` to
`.github/scripts/test-release-contracts.sh` without an alias. The new suite tests:

- raw package version equals Cargo metadata's unique `rclean-cli` version;
- tag exact, missing argument/prefix, empty/malformed and mismatch behavior;
- current changelog extraction succeeds and is non-empty;
- missing version and a synthetic empty section fail;
- no manifest edits, tags or GitHub calls.

Update `.github/workflows/release.yml`:

1. Expand `pull_request.paths` to workflow, scripts, Cargo manifest/lockfile and changelog.
2. Run `test-release-contracts.sh` in PR preflight.
3. In `release-notes-dry-run`, install Rust, verify jq, obtain current version from
   `package-version.sh`, and call the shared extractor. The suite already owns negative fixtures.
4. In the tag `release` job, replace inline AWK/empty checks with the same extractor using
   `${GITHUB_REF_NAME#v}` after the existing preflight/build chain.
5. Preserve all build/package/upload/release/tap steps and event gates otherwise.

## Product-to-Test Mapping

| Invariant | Evidence |
| --- | --- |
| B-001 trigger inputs | parsed YAML path set |
| B-002 version helper | focused raw-version and tool/metadata failures |
| B-003 no duplicated selection | verifier/test call helper; structural `rg` audit |
| B-004 shared extractor | success plus missing/empty contract cases |
| B-005 current PR notes | workflow calls package helper then extractor; no `0.2.0` literal |
| B-006 tag notes unchanged | tag job passes tag-derived version to shared extractor and same body path |
| B-007 suite coverage | ShellCheck/Bash execution output |
| B-008 graph/matrix/gates | parsed YAML plus current-head GitHub checks |
| B-009 scope/full gates | exact manifest, stable/MSRV/VibeGuard/PR evidence |

## Planned Changes Manifest

| Path | Change |
| --- | --- |
| `.github/workflows/release.yml` | Expand PR inputs and use shared version/notes scripts. |
| `.github/scripts/package-version.sh` | Add single raw Cargo package-version source. |
| `.github/scripts/extract-release-notes.sh` | Add shared fail-closed notes extraction. |
| `.github/scripts/verify-release-version.sh` | Replace inline metadata selection with version helper. |
| `.github/scripts/test-release-contracts.sh` | Replace/extend GH268 test suite with version + notes contracts. |
| `.github/scripts/test-verify-release-version.sh` | Delete after rename; no compatibility alias. |

No other implementation path is permitted.

## Verification Plan

```sh
shellcheck .github/scripts/*.sh
bash -n .github/scripts/*.sh
bash .github/scripts/test-release-contracts.sh
git diff --check
git diff --name-only origin/main...HEAD
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

Parse workflow YAML locally to assert paths, five targets and the needs graph. Run all VibeGuard
Rust guards. Require normal CI plus Release preflight, notes dry-run and all five target builds on
the implementation head; require draft release and Homebrew jobs to be SKIPPED on the PR event.

## Risks And Mitigations

- **Version output contamination:** helper prints raw version only; diagnostics use stderr.
- **Partial notes file:** extractor writes a temp sibling and moves only after validation.
- **Heading prefix collision:** exact `## <version>` heading match, not prefix match.
- **PR cost expansion:** only direct release inputs trigger the expensive matrix.
- **Tag behavior drift:** retain tag-derived version and `body_path: release-notes.md` unchanged.
- **Alias drift:** remove the old test filename and update its only workflow caller atomically.
- **Scope creep:** reject permissions, dependency, packaging, runtime or token changes.

## Rollback

Revert the implementation commit. Scripts and PR checks create no release, tag, formula or package;
tag-only external-effect gates remain unchanged.

## Human Gates

- Spec and implementation remain separate PRs.
- Tag creation, draft release publication and crates.io publication remain human actions.
- Merge only after current-head required checks, expected skips, review threads, merge state, scope
  and contract evidence are green.
- The user has provided standing merge authorization; never force push.
