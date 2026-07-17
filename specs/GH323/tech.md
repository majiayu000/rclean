# Deterministic Fake Docker Report Tests - Tech Spec

## Linked Artifacts

- Issue: `#323`
- Product: `specs/GH323/product.md`
- Tasks: `specs/GH323/tasks.md`
- Route: `write_spec`

## Root Cause Evidence

| Area | Evidence on `965cce9` | Decision |
| --- | --- | --- |
| Three non-timeout fake report tests | inherit production 5s timeout | Give only these invocations bounded 30s test headroom. |
| Sequential targeted runs | 10/10 pass | Flake requires contention; a single green run is insufficient. |
| 12 concurrent targeted runs | 6/12 fail in one batch | Use cross-process stress as regression gate. |
| Detailed concurrent output | lines 55/105/176 receive `timedOut`, 5000ms | Root cause is deadline preemption, not changed expected result. |
| Fixture isolation | distinct `TempDir` and scripts per test | No shared path/data race to serialize. |
| Dedicated timeout test | explicit 1s plus `sleep 5` | Preserve as the timeout behavior test. |
| Duplicate search | Docker feature work only | Use #323. |

## Design

Modify exactly three existing `.args(...)` calls in `tests/docker_report_cli.rs`:

```rust
.args(["docker", "report", "--json", "--timeout", "30s"])
```

Apply this only to permission-denied, success/report-only and oversized-output tests. Do not introduce a helper:
three identical arrays are the explicit test inputs and abstraction would add no behavior. Do not add a mutex:
the reproduction uses multiple test processes, and every fixture already owns a distinct temporary directory.

## Product-to-Change Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001 | three exact args additions | source counts + word diff + focused tests |
| B-002 | timeout test untouched | exact source checks + focused timeout assertion |
| B-003 | no `src/` change | name-only diff + exact constant check |
| B-004 | scripts/assertions untouched | word diff + test-integrity review |
| B-005 | one-file three-site manifest | numstat/name-status checks |
| B-006 | sequential and cross-process stress | fresh loop exit codes |
| B-007 | full gates | fresh local/remote evidence |

## Planned Change Manifest

| Path | Change |
| --- | --- |
| `tests/docker_report_cli.rs` | Add `--timeout 30s` to exactly three non-timeout fake report invocations. |

No production, dependency, workflow, fixture body, assertion, config, schema, documentation or security-policy
path is permitted in the implementation diff.

## Exact Source Proof

After implementation:

```sh
test "$(rg -c 'args\(\["docker", "report", "--json", "--timeout", "30s"\]\)' tests/docker_report_cli.rs)" -eq 3
test "$(rg -c 'args\(\["docker", "report", "--json", "--timeout", "1s"\]\)' tests/docker_report_cli.rs)" -eq 1
rg -n 'DEFAULT_TIMEOUT: Duration = Duration::from_secs\(5\)' src/docker.rs
git diff --name-only origin/main...HEAD
git diff --word-diff=porcelain origin/main...HEAD -- tests/docker_report_cli.rs
```

The word diff must show only three additions of `--timeout` and `30s`; the timeout-test line and all assertions
must remain unchanged.

## Stress Verification

```sh
for iteration in $(seq 1 10); do
  cargo test --test docker_report_cli --quiet || exit 1
done
for round in 1 2 3; do
  seq 1 12 | xargs -P12 -I{} sh -c 'cargo test --test docker_report_cli --quiet' || exit 1
done
```

The pre-fix 12-way command exits non-zero when any child test process fails (GNU `xargs` commonly returns 123;
macOS/BSD `xargs` returns 1). After the fix all three rounds must exit zero. This is bounded verification, not a
retry mechanism; every run is required to pass.

## Risks And Mitigations

- Hidden assertion weakening: exact word diff + test-integrity/test-weakening guards + independent review.
- Timeout test accidentally relaxed: exact 1s count and `timeoutMs == 1000` source/test checks.
- Production contract changed: reject any `src/` diff and require exact 5s constant.
- Hang takes longer to fail: 30s is finite and limited to three fake-test invocations.
- Stress command masks failures: require `xargs` exit zero; do not append `|| true` or rerun failed children.
- Cross-platform drift: arguments are CLI strings; require Ubuntu/macOS/Windows current-head CI.

## Verification Plan

```sh
cargo test --test docker_report_cli -- --nocapture
# exact source and stress proofs above
git diff --check
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

Run all eight Rust VibeGuard guards plus test-integrity, test-weakening and dependency guards. Preserve the three
existing production unwrap/expect observations and single-crate workspace skip. Require SpecRail required gate,
valid signatures, independent review, zero reviewThreads, CLEAN merge state and four-check current-head CI.

## Rollback

Remove the three test-only `--timeout 30s` argument pairs. No runtime binary, dependency, schema, config or data
migration is involved.

## Human Gates

- Spec and implementation remain separate PRs.
- Implementation waits for the Spec merge and latest `origin/main`.
- Any need to modify production Docker process/safety behavior becomes separate maintainer-reviewed work.
- Merge only after all fresh gates and standing authorization; never force push.
