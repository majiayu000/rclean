# Release Tag Version Preflight - Tech Spec

## Linked Artifacts

- GitHub issue: `#268`
- Product spec: `specs/GH268/product.md`
- Tasks: `specs/GH268/tasks.md`
- Route: `write_spec`

## Codebase Context

| Area | Evidence | Decision |
| --- | --- | --- |
| `.github/workflows/release.yml` | four steps derive release identity from `GITHUB_REF_NAME#v`; no Cargo equality gate | Add one early preflight job and gate `build` on it. |
| `Cargo.toml` | package `rclean-cli` declares the compiled binary version | Read through `cargo metadata`; do not parse TOML with regex. |
| release dependency graph | `release` needs `build`; `bump-tap` needs `release` | Preserve the graph so one new `build -> preflight` edge gates all external effects. |
| release PR behavior | PRs execute five build targets with `0.0.0-dryrun` packaging | Do not compare the PR ref; run contract tests instead. |
| repository scripts | no existing shell-script location or version helper | Create the smallest `.github/scripts/` helper pair; do not add a general framework. |

## Proposed Design

Add `.github/scripts/verify-release-version.sh`. The script:

1. requires exactly one tag argument;
2. requires the argument to start with `v` and contain a non-empty version;
3. calls `cargo metadata --no-deps --format-version 1` and selects the unique
   `rclean-cli` package version with `jq`;
4. fails if metadata, `jq`, package selection or uniqueness is invalid;
5. compares the full input tag with `v${package_version}` and prints an explicit mismatch error;
6. prints the verified version only after success.

Add `.github/scripts/test-verify-release-version.sh`. It obtains the current package version from
Cargo metadata solely to construct the positive test input, then invokes the production helper to
prove:

- `v<current version>` succeeds;
- `<current version>` fails because the `v` prefix is missing;
- `v0.0.0-version-contract-mismatch` fails because it differs from the package.

The test-side metadata read is not used by the workflow's release decision. The test must reject an
unexpected success and preserve the helper's stderr for diagnosis. It must not edit `Cargo.toml`,
create tags or access GitHub.

Add a `release-preflight` job to `.github/workflows/release.yml`:

- checkout plus the stable Rust toolchain provide the same manifest/tooling used by release builds;
- install or verify `jq` availability explicitly before invoking the helper;
- on `pull_request`, run the contract test script;
- on tag `push`, run the helper with `GITHUB_REF_NAME`;
- make `build` declare `needs: release-preflight` while preserving its existing matrix and steps.

Do not pass a synthetic release version from preflight to later jobs. Existing tag steps continue to
derive their label from `GITHUB_REF_NAME`; preflight establishes that this value is identical to the
Cargo version before they can execute.

## Product-to-Test Mapping

| Invariant | Evidence |
| --- | --- |
| B-001 exact equality | focused helper success for `v<metadata version>` |
| B-002 invalid inputs fail | missing-prefix and mismatch contract cases plus explicit argument guards |
| B-003 single validator | workflow calls helper; no new inline metadata comparison |
| B-004 dependency gate | structural YAML audit of `build.needs` and existing downstream `needs` |
| B-005 PR dry-run | PR condition runs contract tests; current five-target build jobs pass |
| B-006 fail closed | `set -euo pipefail`, exact one-package check, no warning fallback |
| B-007 human gates unchanged | diff review of release/tap/publish steps |
| B-008 bounded scope/gates | exact manifest plus focused/full/MSRV/VibeGuard/current-head CI evidence |

## Planned Changes Manifest

| Path | Change |
| --- | --- |
| `.github/workflows/release.yml` | Add preflight job/conditions and make `build` depend on it. |
| `.github/scripts/verify-release-version.sh` | Implement fail-closed tag/package equality check. |
| `.github/scripts/test-verify-release-version.sh` | Exercise exact-match and negative contracts without external writes. |

No other implementation path is permitted.

## Verification Plan

```sh
bash -n .github/scripts/verify-release-version.sh
bash -n .github/scripts/test-verify-release-version.sh
bash .github/scripts/test-verify-release-version.sh
git diff --check
git diff --name-only origin/main...HEAD
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

Run all installed VibeGuard Rust guards. For the implementation PR, require the release workflow's
preflight, release-notes dry-run and all five target builds to pass on the current head, in addition
to the normal Ubuntu/macOS/Windows/MSRV CI.

## Risks And Mitigations

- **PR refs are not tags:** event-specific conditions keep equality validation tag-only and run
  deterministic helper tests on PRs.
- **Manifest parsing drift:** use Cargo's JSON metadata, not line parsing.
- **Multiple/missing package matches:** require exactly one `rclean-cli` metadata entry.
- **Dependency graph bypass:** `build` explicitly needs preflight; downstream jobs retain their
  existing needs chain.
- **Tool availability:** install Rust explicitly and verify `jq` before helper execution.
- **Shell portability:** helper is used only under `shell: bash` on Ubuntu preflight and is syntax
  checked locally.
- **Scope creep:** do not change packaging, release notes, tap rendering, tokens or runtime code.

## Rollback

Revert the implementation commit. No tag, release, package, formula, runtime schema or persistent
state is created by the preflight itself.

## Human Gates

- Spec and implementation remain separate PRs.
- Tag creation, draft-release publication and crates.io publication remain human actions.
- Merge only after current-head CI, release dry-run, unresolved-thread, merge-state, scope and
  contract evidence are green.
- The user has provided standing merge authorization; never force push.
