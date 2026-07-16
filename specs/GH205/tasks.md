# GH205 Tasks

## Linked Artifacts

- Issue: `#205`
- Product spec: `specs/GH205/product.md`
- Tech spec: `specs/GH205/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH205 Spec PR to merge.

## SpecRail Checklist

- [ ] `SP205-T1` | Owner: `tests` | Done when: all fake Go subprocess tests share a fallible test-only lock and normal fixtures have a bounded 30s deadline | Verify: focused source and behavior checks
- [ ] `SP205-T2` | Owner: `tests` | Done when: original assertions remain intact and lifecycle failures print the observed error | Verify: diff review and focused tests
- [ ] `SP205-T3` | Owner: `verification` | Done when: stress, full stable/MSRV, VibeGuard, CI and PR gates pass with a test-module-only diff | Verify: fresh local and remote evidence

## Implementation Tasks

### SP205-T1 — Serialize fake Go subprocess fixtures

- Owner: `tests`
- Dependencies: merged GH205 Spec PR; latest `origin/main`
- Covers: B-001, B-002, B-003, B-006
- Change: add a static standard-library mutex and fallible guard helper inside the existing test module;
  acquire the guard for success, nonzero and timeout tests; change the normal fixture timeout from 5s to
  30s while preserving the timeout case at 50ms.
- Done when: all three tests retain the guard for their entire fixture lifecycle, poisoning is explicit, normal
  fixtures have finite load headroom, and no production line changes.
- Verify:
  - `rg -n 'FAKE_GO_TEST_LOCK|MutexGuard|Duration::from_secs\(30\)|Duration::from_millis\(50\)' src/clean/deletion.rs`
  - `rg -n 'GO_CLEAN_MODCACHE_TIMEOUT: Duration = Duration::from_secs\(60\)' src/clean/deletion.rs`
  - `cargo test --bin rclean fake_go_modcache -- --nocapture`

### SP205-T2 — Preserve assertions and expose observed errors

- Owner: `tests`
- Dependencies: SP205-T1
- Covers: B-003, B-004, B-005, B-006
- Change: keep every existing success/nonzero/timeout predicate and attach the full observed error to all
  seven nonzero/timeout assertions whose predicate reads `err`, including wrapper context and path checks.
- Done when: expected substrings are unchanged, no assertion is deleted or broadened, and future mismatch logs
  contain the actual wrapper/process error.
- Verify:
  - `git diff --word-diff=porcelain origin/main...HEAD -- src/clean/deletion.rs`
  - `test "$(rg -c 'unexpected error: \{err\}' src/clean/deletion.rs)" -eq 7`
  - `cargo test --bin rclean fake_go_modcache -- --nocapture`
  - VibeGuard test integrity/weakening checks

## Verification And Handoff Tasks

### SP205-T3 — Prove deterministic scope and merge readiness

- Owner: `verification`
- Dependencies: SP205-T1, SP205-T2
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007
- Done when: focused group passes 100 consecutive repetitions, full binary suite passes 10 consecutive
  repetitions, implementation diff is test-module-only, all stable/MSRV/VibeGuard gates pass, and the final
  head has independent review plus green CI/SpecRail/PR gates.
- Verify:
  - `for iteration in $(seq 1 100); do cargo test --bin rclean fake_go_modcache --quiet || exit 1; done`
  - `for iteration in $(seq 1 10); do cargo test --bin rclean --quiet || exit 1; done`
  - `git diff --check`
  - `git diff --name-only origin/main...HEAD`
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - all eight installed Rust VibeGuard guards
  - SpecRail check, required PR gate, signatures, current-head CI and reviewThreads

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007}`
- Missing invariants: `none`

## Handoff Notes

- Implementation file: `src/clean/deletion.rs`, existing `#[cfg(test)]` module only.
- Preserve Unix and Windows fixture bodies and every original assertion predicate.
- Do not change production process/deletion behavior. If production change becomes necessary, stop and route it
  as separate maintainer-reviewed work.
- Merge only with fresh current-head gates under standing authorization; never force push.
