# GH268 Tasks

## Linked Artifacts

- Issue: `#268`
- Product spec: `specs/GH268/product.md`
- Tech spec: `specs/GH268/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH268 Spec PR to merge.

## Implementation Tasks

### SP268-T1 — Capture release identity and dependency baselines

- Owner: `implementation`
- Dependencies: merged GH268 Spec PR; latest `origin/main`
- Covers: B-003, B-004, B-005, B-007
- Change: record every `GITHUB_REF_NAME` consumer, current five-target PR build matrix, and the
  existing `release -> build` / `bump-tap -> release` needs graph.
- Done when: evidence can detect a bypass or unintended release behavior change.
- Verify: source inspection plus focused `rg`/YAML structure queries.

### SP268-T2 — Implement the fail-closed version validator

- Owner: `implementation`
- Dependencies: SP268-T1
- Covers: B-001, B-002, B-003, B-006, B-008
- Change: add `.github/scripts/verify-release-version.sh` using Cargo metadata, exact package
  selection, strict `v<version>` equality and explicit errors.
- Done when: exact current tag succeeds and missing/malformed/mismatched inputs fail without
  warning fallback or repository mutation.
- Verify: `bash -n` and direct positive/negative invocations.

### SP268-T3 — Add deterministic contract coverage

- Owner: `implementation`
- Dependencies: SP268-T2
- Covers: B-001, B-002, B-005, B-006, B-008
- Change: add `.github/scripts/test-verify-release-version.sh` for exact-match, missing-prefix and
  mismatch cases using the production helper.
- Done when: the test fails if any negative case unexpectedly succeeds and passes without network or
  manifest edits.
- Verify: `bash -n` plus `bash .github/scripts/test-verify-release-version.sh`.

### SP268-T4 — Gate the release workflow

- Owner: `implementation`
- Dependencies: SP268-T2, SP268-T3
- Covers: B-003, B-004, B-005, B-006, B-007, B-008
- Change: add tag/PR-aware `release-preflight`, invoke the repository scripts, and make `build`
  depend on it without changing the existing matrix or downstream release/tap logic.
- Done when: tag mismatch stops before build, PR runs contract tests, and the dependency graph has no
  external-effect bypass.
- Verify: workflow diff/structure audit and implementation PR release workflow run.

## Verification And Handoff Tasks

### SP268-T5 — Full gate, VibeGuard and SpecRail audit

- Owner: `verification`
- Dependencies: SP268-T1, SP268-T2, SP268-T3, SP268-T4
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008
- Done when: focused shell, stable/release/MSRV/VibeGuard, normal CI and release-workflow PR jobs pass
  on the current head, with no spec mismatch or extra scope.
- Verify:
  - `bash -n .github/scripts/verify-release-version.sh`
  - `bash -n .github/scripts/test-verify-release-version.sh`
  - `bash .github/scripts/test-verify-release-version.sh`
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - all installed VibeGuard Rust guards

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008}`
- Missing invariants: `none`

## Handoff Notes

- Do not parse `Cargo.toml` with regex or duplicate version comparison in YAML.
- Do not validate PR refs as release tags.
- Do not change the five-target matrix, packaging, changelog extraction, tap rendering or token
  behavior.
- Start implementation from the merged Spec PR on latest `origin/main`.
