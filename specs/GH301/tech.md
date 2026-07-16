# Node and Python Classifier Test Matrix - Tech Spec

## Linked Artifacts

- GitHub issue: `#301`
- Product spec: `specs/GH301/product.md`
- Tasks: `specs/GH301/tasks.md`
- Route: `write_spec`

## Root Cause Evidence

| Area | Current evidence | Decision |
| --- | --- | --- |
| `src/rules/node.rs` | 8 positive rule ids across safe/caution branches; line coverage 38.57% | Add one complete positive JSON matrix and one marker-missing matrix. |
| `src/rules/python.rs` | valid/invalid virtualenv split plus five cache/tox rules; line coverage 26.76% | Add valid/cache, invalid-venv, and marker-missing matrices. |
| `tests/rules/project_artifacts.rs` | Existing project artifact E2E home; 157 lines | Extend this file; do not create a parallel test module. |
| Existing assertions | Several tests use string containment for one rule | New matrix tests parse JSON and compare exact ruleId/safety maps. |
| `tests/rules/common.rs` | Shared directory and one-rule helpers, no JSON contract helper | Keep new matrix helper local because only this module needs it. |

## Design

Add a private test helper in `tests/rules/project_artifacts.rs`:

```rust
fn scan_rule_safety(root: &Path) -> BTreeMap<String, String> {
    let output = Command::cargo_bin("rclean")
        // scan root as JSON with min-size zero
        // assert process success and parse stdout
    // flatten projects[].candidates[] and collect ruleId -> safety
}
```

The helper must fail clearly if stdout is invalid JSON, `projects`/`candidates` has the wrong
shape, or a candidate lacks `ruleId`/`safety`. Inserting an already-seen rule id must assert the
safety is identical and the rule is not duplicated; no candidate order is assumed.

Use existing `make_dir` for non-empty candidate directories. Where a virtualenv marker is needed,
write `pyvenv.cfg` inside the candidate after creation. Fixture construction stays test-local and
does not introduce shell scripts, environment mutation, clocks, sleeps, or platform-specific paths.

## Test Cases

### Node positive matrix

- root marker: `package.json`
- candidate directories: `node_modules`, `.next`, `.turbo`, `.vite`, `.parcel-cache`, `build`,
  `dist`, `out`
- exact map: five safe rules and three caution rules from B-001/B-002

### Node marker-missing matrix

- same eight directories, no `package.json`
- exact filtered `node.*` map is empty

### Python valid/cache matrix

- root marker: `pyproject.toml`
- `.venv` and `venv` each contain `pyvenv.cfg`
- add `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.tox`
- exact map: six safe rules plus `python.tox` caution

### Python invalid and marker-missing matrix

- Python root contains `.venv` and `venv` without a virtualenv marker: only blocked
  `python.venv_plain` appears
- separate root has all Python candidate names but no Python project marker: filtered
  `python.*` map is empty

## Product-to-Test Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001 Node rule set | positive Node fixture | exact map equality |
| B-002 Node safety/rejection | positive + no-marker fixtures | exact map safety and empty filtered map |
| B-003 virtualenv safe paths | valid Python fixture | exact `.venv`/`venv` rule+safety entries |
| B-004 Python cache/tox safety | valid/cache fixture | exact map equality |
| B-005 invalid virtualenv split | invalid Python fixture | exact singleton blocked map |
| B-006 Python marker rejection | no-marker fixture | empty filtered `python.*` map |
| B-007 structured order-independent assertions | local JSON helper | focused test review + deterministic maps |
| B-008 scope/full gate | one-file manifest | diff scope, stable/MSRV/VibeGuard/CI/PR gate |

## Planned Change Manifest

| Path | Change |
| --- | --- |
| `tests/rules/project_artifacts.rs` | Add local JSON rule/safety collector and Node/Python matrix tests. |

No `src/`, helper module, dependency, workflow, documentation, classifier, marker, scan, safety,
ActionPlan, clean/delete, or private-advisory file is permitted in the implementation diff.

## Risks And Mitigations

- **False precision from ordering:** compare `BTreeMap`, not array order.
- **Duplicate rules hidden by map insertion:** reject duplicate rule ids while collecting.
- **Substring false positives:** parse JSON and compare complete filtered ecosystem maps.
- **Cross-ecosystem candidates:** filter only `node.*` or `python.*`, then compare the complete
  expected ecosystem map.
- **Coverage gaming:** coverage is discovery/review evidence; acceptance is behavioral matrix,
  not a numeric CI threshold.
- **Behavior drift:** production diff is forbidden and full existing suite remains required.

## Verification Plan

```sh
cargo test --test rules project_artifacts::node_classifier_matrix
cargo test --test rules project_artifacts::python_classifier_matrix
cargo test --test rules
CARGO_TARGET_DIR=/tmp/rclean-cov-301 cargo llvm-cov --all-features --summary-only
git diff --check
git diff --name-only origin/main...HEAD
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

Coverage verification records the updated `rules/node.rs` and `rules/python.rs` report rows and
reviews uncovered-line output for reachable classifier arms. It does not add a wall-clock or
percentage assertion to the repository.

## Rollback

Revert the test-only implementation commit. No production behavior, schema, dependency, data or
migration is changed.

## Human Gates

- Spec and implementation remain separate PRs.
- Implementation starts only after the Spec PR merges on the latest `origin/main`.
- Any discovered classifier behavior change stops this issue and requires a new decision; this
  issue may only add tests for current behavior.
- Merge only after current-head CI, independent review, zero unresolved review threads, clean
  merge state, valid signatures and the user's standing authorization; never force push.
