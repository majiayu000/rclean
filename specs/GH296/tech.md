# Project-Scoped Risk Score Reuse - Tech Spec

## Linked Artifacts

- GitHub issue: `#296`
- Product spec: `specs/GH296/product.md`
- Tasks: `specs/GH296/tasks.md`
- Route: `write_spec`

## Root Cause Evidence

| Area | Evidence | Decision |
| --- | --- | --- |
| `build_project_report` | `compute_risk_score` 位于 candidate loop 内 | 在 filter 后通过 call-local lazy cell 初始化。 |
| `compute_risk_score` | 每次调用都执行 `has_lockfile(project_dir)` | standalone helper 保持不变；仅 report materialization 复用最终值。 |
| risk inputs | git/activity/project path 对同一 report 不变 | 明确视为 project-scoped value。 |
| 1,000×8 fixture | 8,000 candidates，无 lockfile 时约 96,000 probes | 目标约 12,000 probes，交错基准验证 wall time。 |
| current benchmarks | 每项目通常只有一个 candidate | 增加 multi-candidate shape，保留全部现有 shapes。 |

## Design

在 `build_project_report` 的 candidate loop 前创建单线程、函数调用局部的 lazy cell：

```rust
let risk_score = OnceCell::new();

for (mut draft, bytes) in drafts.into_iter().zip(size_summary.candidate_bytes) {
    // existing safety demotion and min-size filter stay in the same order
    if /* existing filter */ {
        continue;
    }

    let candidate_risk_score = cached_project_risk_score(&risk_score, || {
        compute_risk_score(git.as_ref(), activity_time, dir)
    });
    // existing candidate materialization
}
```

`std::cell::OnceCell<f32>` 足够，因为 report materialization loop 是串行的；不需要 `Sync`、
mutex、全局状态或新依赖。一个私有 helper 接受 `FnOnce() -> f32` 并返回 copied `f32`，让
focused test 能用计数闭包验证首次调用一次、后续闭包不执行。若零候选通过 filter，则代码
不会调用 helper，cell 保持未初始化。

不要改变 `compute_risk_score` 或 `has_lockfile` 的签名/内部语义。`explain` 和直接测试仍经
由该函数独立读取当前状态。cache 生命周期严格限于一次 `build_project_report` 调用。

## Test Design

`src/scan/project.rs` private tests 增加 lazy cache contract：

- 未调用 helper 时计数为 0；
- 第一次调用执行闭包并缓存值；
- 第二次传入会 panic 或改变计数的闭包，证明它不执行且返回首值；
- 多候选 scan fixture 保持相同候选顺序与相同 risk 值。

既有 `scan::tests` 已覆盖 risk formula 的 lockfile/recent/dirty 组合，继续作为 B-003/B-004
回归门。不得削弱这些断言。

## Benchmark Design

在 `benches/scan_throughput.rs` 增加 bounded fixture，例如固定数量 Node projects，每个含
`package.json`、一个 source file 和八个现有候选：`node_modules`、`.next`、`.turbo`、
`.vite`、`.parcel-cache`、`dist`、`build`、`out`。fixture 构造必须在 Criterion timed
closure 外；使用已有 release binary、`--json --min-size 0`，并新增独立 benchmark name。

发现阶段固定 1,000×8 fixture 保持不变。baseline/implementation binaries 在同一 session
按每对奇偶轮换先后顺序交错运行，至少收集 31 个 warmed pairs；报告各自 range/median、
每对 speedup、paired median 和 win count。接受条件为 paired median speedup `>=8%` 且
implementation 至少赢得三分之二配对。两组 calibration evidence 为：首轮 15 对 paired
median 12.96%，复核 31 对为 10.42%，后者 implementation 赢 25/31。三个既有 benchmark
shapes 与 marker-heavy shape after point estimate 均不得超过 baseline 1.10x。不得在
CI/test 中写 wall-clock assertion。

## Output Equivalence

对同一固定 fixture 分别运行 baseline 和 implementation release binary，不在两次运行间
修改 fixture。删除顶层 `scannedAt` 后递归排序 JSON keys，要求 diff 为空。比较包含
projects、markers、candidate fields、risk、warnings、summary 和 ordering。

7 天边界是唯一被明确收敛的时间一致性：旧实现理论上可能在一个循环中给同项目候选不同
risk，新实现固定为一次 report-level snapshot；常规固定 activity fixture 输出必须完全相同。

## Product-to-Test Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001 zero evaluation | filter 后调用 lazy helper | zero-call counter + source placement review |
| B-002 once/reuse | call-local `OnceCell<f32>` | first/second closure counter test |
| B-003 unchanged inputs/formula | unchanged `compute_risk_score` | existing formula tests + scoped diff |
| B-004 standalone/explain | helper unchanged | existing scan/explain tests |
| B-005 materialization compatibility | loop order unchanged | focused/full tests + JSON diff |
| B-006 output equivalence | no schema/data changes | normalized same-fixture diff |
| B-007 durable benchmark | new fixture/bench function | Criterion lists all shapes |
| B-008 performance | one init per project | static probe count + interleaved measurements |
| B-009 full gate/scope | two-file manifest | stable/MSRV/VibeGuard/CI/PR gate |

## Planned Change Manifest

| Path | Change |
| --- | --- |
| `src/scan/project.rs` | Add call-local lazy risk reuse and focused zero/once contract tests. |
| `benches/scan_throughput.rs` | Add bounded multi-candidate-per-project benchmark shape. |

No classifier, safety, sizing, git/activity, output, schema, CLI, ActionPlan, clean/delete,
dependency, workflow, documentation or private-advisory file is in implementation scope.

## Risk Analysis

- **Over-eager I/O:** initialization remains after the existing filter; all-filtered reports do no
  risk work.
- **Stale cache:** cell is stack-local to one report and cannot cross project or scan boundaries.
- **Behavior drift:** standalone formula/helper stay unchanged; normalized JSON and existing tests
  lock ordinary output.
- **Timing boundary:** one report intentionally uses one risk snapshot, eliminating candidate-level
  inconsistency around the 7-day threshold.
- **Benchmark noise:** use odd/even-order interleaved same-session pairs, paired median/win count,
  and structural probe-count evidence; do not substitute isolated medians for the paired gate.

## Verification Plan

```sh
cargo test scan::project::tests
cargo test scan::tests
cargo bench --bench scan_throughput -- --noplot
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

Revert the implementation commit. Report materialization resumes per-candidate risk computation;
there is no schema, dependency, persistent state or migration to undo.

## Human Gates

- Spec and implementation remain separate PRs.
- Implementation starts only after this Spec PR merges on latest `origin/main`.
- Any classifier, safety, ActionPlan, clean/delete or cross-scan cache change stops this issue and
  requires a new decision.
- Merge only after current-head CI, zero unresolved review threads, clean merge state, output and
  performance evidence under the user's standing authorization; never force push.
