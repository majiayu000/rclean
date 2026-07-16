# GH271 Tasks

## Linked Artifacts

- Issue: `#271`
- Product spec: `specs/GH271/product.md`
- Tech spec: `specs/GH271/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH271 Spec PR to merge.

## Implementation Tasks

### SP271-T1 — Capture trigger, version, extractor and graph baselines

- Owner: `implementation`
- Dependencies: merged GH271 Spec PR; latest `origin/main`
- Covers: B-001, B-003, B-004, B-005, B-006, B-008
- Change: record PR paths, hardcoded version, duplicate AWK bodies, script callers, five targets and
  downstream needs/conditions.
- Done when: evidence detects every intended replacement and any matrix/gate drift.
- Verify: source inspection, PR #213 check history and parsed workflow structure.

### SP271-T2 — Centralize Cargo package version

- Owner: `implementation`
- Dependencies: SP271-T1
- Covers: B-002, B-003, B-007, B-009
- Change: add `package-version.sh`; refactor tag verifier and contract tests to call it.
- Done when: raw output is exactly the unique package version, failures are explicit, and no copied
  metadata selection remains outside the helper.
- Verify: ShellCheck, Bash syntax, focused success/tool/metadata cases and `rg` audit.

### SP271-T3 — Centralize and test release-note extraction

- Owner: `implementation`
- Dependencies: SP271-T1
- Covers: B-004, B-005, B-006, B-007, B-009
- Change: add atomic fail-closed extractor; rename the test suite and add current/missing/empty notes
  cases; remove the old test filename.
- Done when: PR/tag can share the helper and all notes contracts pass without repository mutation.
- Verify: ShellCheck, Bash syntax, focused suite, file/hash/status before-after checks.

### SP271-T4 — Wire release-input PR coverage

- Owner: `implementation`
- Dependencies: SP271-T2, SP271-T3
- Covers: B-001, B-003, B-005, B-006, B-008, B-009
- Change: expand paths, update the preflight suite caller, use current package version in PR notes,
  and replace tag AWK with the shared extractor.
- Done when: no hardcoded `0.2.0` or duplicate extractor remains and matrix/needs/gates are unchanged.
- Verify: parsed YAML, diff review and implementation PR Release workflow run.

## Verification And Handoff Tasks

### SP271-T5 — Full gate, VibeGuard and SpecRail audit

- Owner: `verification`
- Dependencies: SP271-T1, SP271-T2, SP271-T3, SP271-T4
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009
- Done when: focused shell, stable/release/MSRV/VibeGuard, 11 SUCCESS and 2 expected SKIPPED checks
  pass on the current head with no spec mismatch or extra scope.
- Verify:
  - `shellcheck .github/scripts/*.sh`
  - `bash -n .github/scripts/*.sh`
  - `bash .github/scripts/test-release-contracts.sh`
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - all installed VibeGuard Rust guards

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009}`
- Missing invariants: `none`

## Handoff Notes

- No inline Cargo version parsing or duplicated AWK extraction in workflow YAML.
- Remove the old test name; do not leave an alias.
- Preserve the five-target matrix and tag-only external-effect gates exactly.
- Start implementation from the merged Spec PR on latest `origin/main`.
