# 并行项目活动时间预计算 - Tech Spec

## Linked Artifacts

- GitHub issue: `#284`
- Product spec: `specs/GH284/product.md`
- Tasks: `specs/GH284/tasks.md`
- Route: `write_spec`

## Codebase Context

| Area | Current evidence | Decision |
| --- | --- | --- |
| `src/scan/mod.rs:209-237` | sorted `project_dirs` 后串行调用 `build_project_report` | 在该循环前按相同向量顺序预计算 activity。 |
| `src/scan/project.rs:32-43` | report builder 内部调用 `project_activity` | 改为接收明确的 `SystemTime`，禁止内部重走。 |
| `src/scan/project.rs:181-207` | 唯一 activity traversal 实现 | 保持函数体不变，由 batch helper 复用。 |
| `src/scan/sizer.rs` | candidate sizing 已使用 Rayon | 不扩大改动；activity 预计算在 sizing 前 collect 完成。 |
| `benches/scan_throughput.rs` | 100 small 与 one huge candidate fixture | 保留两者并新增多项目宽源码 activity shape。 |

## Proposed Internal Design

在 `src/scan/project.rs` 增加一个私有或 `pub(crate)` batch helper：

```rust
pub(crate) fn project_activities(
    project_dirs: &[PathBuf],
    max_depth: usize,
) -> Vec<SystemTime>
```

内部用一个共享的 resolver 保留现有 fallback：

```rust
fn resolved_project_activity(project_dir: &Path, max_depth: usize) -> SystemTime {
    project_activity(project_dir, max_depth).unwrap_or_else(SystemTime::now)
}
```

分支契约：

- `[] => Vec::new()`；
- `[only] => vec![resolved_project_activity(only, max_depth)]`；
- 多项目 => `project_dirs.par_iter().map(...).collect()`。

Rayon indexed parallel iterator 的 `collect::<Vec<_>>()` 保持输入索引顺序，不按完成顺序
排列。实现使用现有 global pool，不创建 pool 或 OS thread。

`build_project_report` 新增 `activity_time: SystemTime` 参数并删除内部 activity lookup。
`scan()` 对 `project_dirs` 排序后调用 batch helper，再用
`project_dirs.into_iter().zip(activity_times)` 串行 materialize。zip 前以测试和构造保证长度
一一对应，不使用可能静默丢项目的手写截断/fallback 路径。

`explain_path_with_activity_depth` 仍是单路径调用，继续直接调用 `project_activity`，不经过
batch helper。

## Product-to-Test Mapping

| Invariant | Implementation area | Deterministic verification |
| --- | --- | --- |
| B-001 empty | `project_activities` match | empty input returns empty vec |
| B-002 single direct | single slice branch | test-only resolver counter or direct-path unit assertion |
| B-003 bounded multi | Rayon `par_iter` | multiple dirs produce exactly N results; code review proves no pool/thread creation |
| B-004 index order | indexed collect + zip | distinct stable mtimes return in input order and scan order regression stays stable |
| B-005 traversal equivalence | unchanged `project_activity` body | batch results equal serial calls on nested/depth/pruned/symlink fixture |
| B-006 fallback | shared resolver | nonexistent project returns recent `SystemTime` and preserves vector slot |
| B-007 one value | `build_project_report` parameter | focused test proves supplied activity drives filter/output/staleness/risk without lookup |
| B-008 materialization | unchanged serial loop body | existing scan, dirty-git, sizing and warning suites stay green |
| B-009 report equivalence | release binaries + fixed fixture | normalized before/after JSON diff is empty except declared `scannedAt` |
| B-010 performance | fixed benchmarks | five warmed runs per revision plus Criterion comparison |

## Planned Changes Manifest

| Path | Change |
| --- | --- |
| `src/scan/project.rs` | Add bounded batch helper/direct path, accept supplied activity in report builder, add focused tests. |
| `src/scan/mod.rs` | Precompute ordered activity vector and zip it into existing serial report materialization. |
| `benches/scan_throughput.rs` | Add a bounded multi-project wide-source fixture/benchmark; preserve existing shapes. |

No Cargo dependency, schema, rule, safety, delete, ActionPlan or documentation file is in the
implementation manifest.

## Concurrency And Safety Notes

- `project_activity` is read-only and has no shared mutable state; each task owns a distinct path.
- Rayon global pool bounds work; do not use `std::thread::spawn`, custom pools or async runtimes.
- Activity collection completes before Git lookup/candidate sizing/report mutation begins, avoiding
  nested report-level concurrency and warning reordering.
- Indexed `par_iter` collection is required. An unordered channel/collector is prohibited.
- Do not make metadata errors louder or quieter in this performance change; retain the exact
  existing activity policy.
- A project may disappear concurrently. Preserve its index and conservative `now` value rather than
  dropping it from zip/report processing.
- This change does not alter destructive or trust-model gates, but output equivalence matters because
  activity contributes to filtering and risk score.

## Benchmark Design

Keep existing `many_small_projects_json` and `one_huge_candidate_json`. Add an activity-focused
fixture with multiple sibling Node projects, a tiny candidate per project and hundreds of
non-candidate source files per project. Fixture creation remains outside the timed closure.

For acceptance evidence, additionally generate the fixed 20 × 2,000 source-file fixture used in
the issue and capture five warmed release runs for `origin/main@a74990d` and implementation on the
same machine with Git disabled. Use median wall time and require at least 20% improvement.

Run the existing 100-small Criterion shape before/after and require after point estimate <= 1.10 ×
before. Record the new activity benchmark as durable trend evidence; do not introduce a flaky CI
wall-clock assertion.

## Output Equivalence

Run both release binaries against one fixture whose source/candidate mtimes are fixed before either
scan. Remove only top-level `scannedAt`, canonicalize JSON key ordering, and require an empty diff.
Do not remove `activity`, `stalenessDays`, `riskScore`, project ordering, warnings, byte totals or
summary fields from the comparison.

## Verification Plan

Focused:

```sh
cargo test scan::project::tests
cargo test scan::tests::scan_sizes_one_large_candidate_and_many_small_projects_deterministically
cargo test scan::tests::dirty_git_marks_candidate_caution
cargo bench --bench scan_throughput -- --noplot
```

Scope and repository gates:

```sh
git diff --check
git diff --name-only origin/main...HEAD
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

## Rollback

Revert the implementation commit. Report building then resumes calling `project_activity` serially;
there is no schema, dependency, persisted state or migration to unwind.

## Human Gates

- Spec and implementation remain separate PRs.
- Merge only after current-head CI, unresolved-thread, merge-state, benchmark, output-diff and
  spec-vs-implementation evidence is green.
- The user has provided standing merge authorization for this optimization run; never force push.
