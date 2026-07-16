# GH301 Tasks

## Linked Artifacts

- Issue: `#301`
- Product spec: `specs/GH301/product.md`
- Tech spec: `specs/GH301/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — implementation waits for the GH301 Spec PR to merge.

## SpecRail Checklist

- [ ] `SP301-T1` | Owner: `implementation` | Done when: the local `--include-blocked` JSON helper returns a duplicate-safe order-independent ruleId-to-safety map and source review confirms explicit shape failures | Verify: `cargo test --test rules project_artifacts::node_classifier_matrix`
- [ ] `SP301-T2` | Owner: `implementation` | Done when: Node positive and marker-missing fixtures prove B-001 and B-002 exactly | Verify: `cargo test --test rules project_artifacts::node_classifier_matrix`
- [ ] `SP301-T3` | Owner: `implementation` | Done when: Python valid/cache, invalid-venv, and marker-missing fixtures prove B-003 through B-006 exactly | Verify: `cargo test --test rules project_artifacts::python_classifier_matrix`
- [ ] `SP301-T4` | Owner: `verification` | Done when: coverage evidence executes reachable classifier arms and the focused/full scope gates pass | Verify: `cargo test --test rules`
- [ ] `SP301-T5` | Owner: `verification` | Done when: stable, release, MSRV, VibeGuard, SpecRail, CI, review-thread, signature, and merge-state gates pass | Verify: `cargo test`

## Implementation Tasks

### SP301-T1 — Add the structured rule/safety collector

- Owner: `implementation`
- Dependencies: merged GH301 Spec PR; latest `origin/main`
- Covers: B-007, B-008
- Change: add the minimal module-local helper that runs
  `rclean scan --json --min-size 0 --include-blocked`, parses projects/candidates, and collects
  duplicate-safe ruleId-to-safety values without order dependence.
- Done when: normal matrix execution passes; source review confirms malformed shape/missing fields
  and duplicate rule ids fail explicitly; the helper has no production or shared-fixture impact.
- Verify:
  - `cargo test --test rules project_artifacts::node_classifier_matrix`
  - focused source review of the JSON shape assertions and duplicate insertion assertion

### SP301-T2 — Lock the complete Node matrix

- Owner: `implementation`
- Dependencies: SP301-T1
- Covers: B-001, B-002, B-007
- Change: add one `package.json` fixture with all eight Node candidates and one equivalent
  marker-missing fixture; compare exact filtered maps.
- Done when: five safe and three caution rules match exactly, and the no-marker map is empty.
- Verify: `cargo test --test rules project_artifacts::node_classifier_matrix`

### SP301-T3 — Lock the complete Python matrix

- Owner: `implementation`
- Dependencies: SP301-T1
- Covers: B-003, B-004, B-005, B-006, B-007
- Change: add valid/cache, invalid virtualenv, and marker-missing fixtures with exact filtered map
  assertions.
- Done when: safe/caution/blocked values match the current classifier, `.venv` and plain `venv`
  preserve their intentional invalid-marker difference, and the no-marker map is empty.
- Verify: `cargo test --test rules project_artifacts::python_classifier_matrix`

## Verification And Handoff Tasks

### SP301-T4 — Prove coverage and one-file scope

- Owner: `verification`
- Dependencies: SP301-T1, SP301-T2, SP301-T3
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008
- Done when: the rules target passes, coverage evidence shows reachable Node/Python classifier
  arms executed, diff is exactly one test file, and no ignore/sleep/time assertion is added.
- Verify:
  - `cargo test --test rules`
  - `CARGO_TARGET_DIR=/tmp/rclean-cov-301 cargo llvm-cov --all-features --summary-only`
  - `git diff --check`
  - `git diff --name-only origin/main...HEAD`
  - `! rg -n '#\[ignore|sleep\(|Duration::' tests/rules/project_artifacts.rs`

### SP301-T5 — Full stable/MSRV/VibeGuard/SpecRail gate

- Owner: `verification`
- Dependencies: SP301-T4
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008
- Done when: focused/default/release/MSRV, all installed VibeGuard Rust guards, SpecRail
  spec-vs-implementation, current-head CI/review-thread/signature/merge-state checks pass.
- Verify:
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

- Implementation file: `tests/rules/project_artifacts.rs` only.
- Reuse existing fixture helpers; keep the new JSON map helper local to this module.
- Do not change production rules to make expectations pass. If current behavior contradicts the
  approved matrix, stop and report the mismatch instead of rewriting either side silently.
- Implementation starts from the merged Spec PR on latest `origin/main`.
- Merge only with fresh current-head gates under standing authorization; never force push.
