# Deterministic Fake Go Subprocess Tests - Tech Spec

## Linked Artifacts

- GitHub issue: `#205`
- Product spec: `specs/GH205/product.md`
- Tasks: `specs/GH205/tasks.md`
- Route: `write_spec`

## Root Cause Evidence

| Area | Evidence | Decision |
| --- | --- | --- |
| PR #212 / commit `727fde49` | Only raised normal fake Go timeout from 1s to 5s. | Treat as incomplete mitigation, not a new issue. |
| `src/clean/deletion.rs` test module | Three subprocess tests are eligible to run concurrently. | Test the remediation hypothesis by adding one test-only static mutex used by all three. |
| timeout fixture | Unix/Windows fixture spins until killed and uses 50ms. | Preserve behavior and deadline while removing this known in-process concurrency source. |
| 2026-07-16 Ubuntu CI | Timeout assertion failed on a README-only head; rerun passed, but the actual error was hidden. | Classify as low-frequency contention evidence, not proof of a specific lifecycle race. |
| local repeated full binary suite | Nonzero case failed after 5.01s; focused group passed 200 rounds. | Increase bounded normal-fixture headroom and require repeated full-suite verification. |
| assertion output | Existing `assert!(err.contains(...))` hides `err`. | Add messages without weakening predicates. |

Search of all GitHub issues, PRs, local specs and history found #205/#212 as the exact prior work and no
open competing implementation. Reopen #205 and use its stable `specs/GH205/` packet rather than creating a
duplicate issue.

## Design

Keep every implementation line inside `src/clean/deletion.rs` `#[cfg(test)] mod tests`:

1. Import `std::sync::{Mutex, MutexGuard}` and declare
   `static FAKE_GO_TEST_LOCK: Mutex<()> = Mutex::new(())`.
2. Add a small `fake_go_test_guard()` helper returning
   `Result<MutexGuard<'static, ()>, Box<dyn std::error::Error>>`. Convert poison to an explicit
   `std::io::Error::other(...)`; never call `unwrap` or silently recover a poisoned guard.
3. Acquire and retain `_guard` at the start of all three fake Go subprocess tests, before creating or
   launching the fixture.
4. Change only `FAKE_GO_TEST_TIMEOUT` from 5 seconds to 30 seconds. The timeout test continues to pass
   `Duration::from_millis(50)` directly.
5. Add the observed `err` as the message on all seven nonzero/timeout assertions whose predicate reads
   `err`: both wrapper-context checks, both path checks, `exited`, `permission denied` and `timed out`.
   Keep every original substring predicate unchanged.

This is one remediation hypothesis rather than a claimed complete causal proof: the mutex removes a known
in-process concurrency source and the 30-second bounded deadline provides shared-runner scheduling headroom.
Repeated full-suite stress is the validation. Neither mechanism is compiled into production because both live
inside the existing `#[cfg(test)]` module.

## Product-to-Change Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001 | static mutex + fallible guard helper + three acquisitions | source checks, focused tests, unwrap guard |
| B-002 | test constant 5s -> 30s; production constant untouched | exact `rg` checks and diff review |
| B-003 | timeout test keeps 50ms and same fake program under guard | focused timeout test and source checks |
| B-004 | retain every success/nonzero assertion | diff review and focused tests |
| B-005 | add observed-error messages to every `err` predicate, keeping all predicates | source/diff review and forced failure readability review |
| B-006 | only `#[cfg(test)]` module changes | name-only diff and production-boundary inspection |
| B-007 | stress/full/MSRV/CI/PR gates | fresh command and remote evidence |

## Planned Change Manifest

| Path | Change |
| --- | --- |
| `src/clean/deletion.rs` | Test-only lock, normal-fixture timeout headroom and diagnostic assertion messages. |

No other source, test, dependency, workflow, config, documentation, schema, cleanup or security-policy file
is permitted in the implementation diff.

## Risks And Mitigations

- **Poisoned lock hides later results:** convert poisoning into an explicit test error with context.
- **Lock lifetime ends too early:** bind the returned guard at the beginning of each full test body.
- **Timeout contract accidentally relaxed:** exact source check requires the timeout case to remain 50ms and
  continue asserting `timed out`.
- **Normal hang takes longer to fail:** keep a finite 30-second test-only bound; production already has its
  independent 60-second bound.
- **Test weakening:** preserve every existing predicate and add diagnostics only; run VibeGuard test-integrity
  guards and independent review.
- **Cross-platform drift:** do not modify Unix or Windows fixture bodies; require three-platform CI.
- **Hypothesis is insufficient:** require repeated complete binary suites; any recurrence with the richer error
  becomes fresh evidence for a separate diagnosis instead of weakening or repeatedly rerunning the test.
- **Scope creep into destructive behavior:** reject any diff outside the test module and require no changes to
  production constants or `run_native_tool`.

## Verification Plan

```sh
cargo test --bin rclean fake_go_modcache -- --nocapture
for iteration in $(seq 1 100); do
  cargo test --bin rclean fake_go_modcache --quiet || exit 1
done
for iteration in $(seq 1 10); do
  cargo test --bin rclean --quiet || exit 1
done
rg -n 'FAKE_GO_TEST_LOCK|MutexGuard|Duration::from_secs\(30\)|Duration::from_millis\(50\)' src/clean/deletion.rs
rg -n 'GO_CLEAN_MODCACHE_TIMEOUT: Duration = Duration::from_secs\(60\)' src/clean/deletion.rs
test "$(rg -c 'unexpected error: \{err\}' src/clean/deletion.rs)" -eq 7
git diff --check
git diff --name-only origin/main...HEAD
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

Run all eight installed Rust VibeGuard guards. The three pre-existing production unwrap/expect observations may
remain but no new observation is allowed. Run SpecRail check and required PR gate; require current-head Ubuntu,
macOS, Windows and MSRV CI success.

## Rollback

Revert the test-only implementation commit. Runtime binaries, cleanup behavior, dependencies, schemas and data
need no migration or rollback.

## Human Gates

- Spec and implementation remain separate PRs.
- Implementation starts only after the Spec PR merges on latest `origin/main`.
- This test-only change does not alter the SECURITY.md trust-model boundary; any discovered need to change
  production process/deletion behavior must stop and become separate maintainer-reviewed work.
- Merge only after current-head CI, independent review, zero unresolved review threads, clean merge state, valid
  signatures, SpecRail gate and the user standing authorization; never force push.
