# Free Machine-Readable Proposal Output - Tech Spec

## Linked Artifacts

- GitHub issue: `#277`
- Product spec: `specs/GH277/product.md`
- Tasks: `specs/GH277/tasks.md`
- Route: `write_spec`

## Codebase Context

| Area | Evidence | Decision |
| --- | --- | --- |
| public flag | `CommonScanArgs.json`; free help promises machine JSON | Honor it for non-interactive free. |
| current flow | all proposal/plan/gap paths call `println!` | Gate human rendering and add one JSON renderer. |
| proposal | private ranked references to existing `Candidate` | Serialize references; do not clone/redeclare candidate fields. |
| ActionPlan | selected candidates written before final exit decision | Preserve write behavior for met and shortfall. |
| no candidate | returns 3 before plan-path selection | Emit JSON with null path, keep no-write behavior. |
| errors | `RcleanError::Output` already converts serde JSON errors | Reuse current error path. |
| E2E fixtures | `tests/cli/free_output.rs` covers human met/unmet/no-candidate | Add JSON peers without weakening existing assertions. |

## Proposed Design

Add a private borrowed serialization view in `src/free.rs`:

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FreeProposalOutput<'a> {
    schema_version: u32,
    target_bytes: u64,
    selected_bytes: u64,
    target_met: bool,
    plan_path: Option<String>,
    candidates: Vec<&'a Candidate>,
}
```

`plan_path` is not skipped so the no-candidate shape contains explicit JSON `null`. Build candidates
from `Proposal.candidates` in their current order. `Candidate` already derives `Serialize` and owns the
canonical camelCase shape.

Extract the existing proposal header/list/total prints into `print_human_proposal`. Add
`print_json_proposal(target, proposal, Option<&Path>) -> Result<(), RcleanError>` that serializes the
complete object before one stdout print. Serialization errors therefore occur before any JSON byte.

Restructure `run` without changing selection:

1. Preserve interactive conflict and terminal gates.
2. Scan and select exactly as today.
3. If no candidates: print JSON with `targetMet=false` only when requested, otherwise the existing
   human message; return 3 even when target is zero, preserving current command behavior.
4. For a non-empty proposal, print the existing human proposal only when JSON is false.
5. Preserve interactive handoff.
6. Resolve selected candidates and plan path; write the ActionPlan before JSON output.
7. JSON mode prints one proposal document. Human mode prints the existing plan hints and, on
   shortfall, the existing gap text.
8. Return 0 or 3 from the unchanged `total_bytes >= target` condition.

Do not route through `output::print_json(ScanReport)` because free output is a different schema and
must expose the selected subset and plan path.

## Product-to-Test Mapping

| Invariant | Evidence |
| --- | --- |
| B-001 pure document | parse complete stdout; reject human substrings |
| B-002 exact top-level shape | sorted key-set assertion plus values/types |
| B-003 Candidate reuse/order | candidate field assertions and selected path match with ActionPlan |
| B-004 met result | exit 0, targetMet true, plan exists and parses |
| B-005 shortfall result | exit 3, targetMet false, non-null plan exists and parses |
| B-006 no candidates | positive/zero targets both exit 3 with false, empty array, zero bytes, null path and no file |
| B-007 fail-before-output | invalid/unwritable plan-path E2E asserts empty stdout and stderr error |
| B-008 interactive conflict | existing E2E remains green |
| B-009 human compatibility | existing met/unmet/no-candidate tests unchanged and green |
| B-010 scope/full gates | exact manifest, stable/release/MSRV/VibeGuard/current-head evidence |

## Planned Changes Manifest

| Path | Change |
| --- | --- |
| `src/free.rs` | Add borrowed JSON view/rendering and branch human vs JSON flow. |
| `tests/cli/free_output.rs` | Add met, shortfall, no-candidate, purity and plan-error E2E coverage. |
| `README.md` | Document free JSON usage and high-level fields. |
| `docs/architecture.md` | Record the independent versioned free proposal schema. |

No other implementation path is permitted.

## Verification Plan

```sh
cargo test --test cli free_output::free_json -- --nocapture
cargo test --test cli free_output -- --nocapture
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
git diff --check
git diff --name-only origin/main...HEAD
```

Run all installed VibeGuard Rust guards. The implementation PR current head must have successful
CI, zero unresolved review threads and CLEAN/MERGEABLE state. Spec-vs-implementation review must
map B-001 through B-010 and reject undeclared JSON fields or output changes.

## Risks And Mitigations

- **second candidate schema:** serialize `&Candidate` directly.
- **partial stdout before plan error:** emit no JSON until plan write succeeds.
- **null/absent drift:** keep `planPath` present as explicit null for no-candidate output.
- **human regression:** extract rather than rewrite current strings; retain existing E2E assertions.
- **zero-target drift:** define target-met as non-empty proposal plus the existing byte threshold;
  retain the current no-candidate exit 3 for zero as well as positive targets.
- **plan/JSON mismatch:** tests compare selected candidate paths and the reported plan path.
- **scope creep:** broken pipe, scan warnings, progress and selection logic are non-goals.

## Rollback

Revert the implementation commit. Existing ActionPlans need no migration because this change adds a
stdout schema only and does not alter persisted content.

## Human Gates

- Spec and implementation remain separate PRs.
- The user has provided standing merge authorization; each merge still requires current-head CI,
  review-thread, merge-state and exact-scope evidence. Never force push.
- No release, publication, deletion or security action is part of this work.
