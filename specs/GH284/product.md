# 并行项目活动时间预计算 - Product Spec

## Linked Issue

- GitHub issue: `#284`
- URL: `https://github.com/majiayu000/rclean/issues/284`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `medium`

## Summary

扫描多个项目时，先使用现有有界 Rayon worker 并行预计算每个项目的活动时间，再按
原有确定性顺序生成项目报告。活动时间算法、输出内容和所有安全分类保持不变，仅消除
独立项目活动遍历之间不必要的串行等待。

## Problem

phase 1 已通过 `ignore::WalkParallel` 并行遍历扫描根中的非候选文件；phase 2 随后按
排序后的项目列表逐个调用 `project_activity`。因此每个项目的非候选源码元数据会被再次
读取，而且这些相互独立的 activity walks 当前完全串行。

在 `origin/main@a74990d` 的 fresh evidence 中：

- Criterion `scan/many_small_projects_json`：25.031ms point estimate，区间
  24.647–25.474ms；
- 20 个项目、每项目 2,000 个源码文件的 fixture（共 40,040 files），禁用 Git 后
  五次 release scan 均为 0.12s；
- 代码路径确认 phase 1 在 `scan/walker.rs` 读取源码 metadata，phase 2 又在
  `scan/project.rs::project_activity` 串行遍历每个项目。

第二次遍历不能直接由 phase-1 size map 替代：scan max depth 相对扫描根，而 activity
max depth 相对项目根。可优化的是多个独立 activity walks 的调度方式。

## Goals

- 两个及以上项目时，用现有 Rayon 全局线程池并行预计算活动时间。
- 零项目返回空结果；单项目继续走直接、无并行调度路径。
- 继续调用唯一的 `project_activity` 实现，不复制 traversal/pruning 逻辑。
- 预计算结果与已排序的 `project_dirs` 按索引稳定对应。
- 报告仍按现有顺序串行生成，projects 最终排序和 warnings 行为不变。
- `--older-than`、`activity.last_modified`、`staleness_days` 和 `risk_score` 使用同一个
  预计算时间值。
- 提供固定多项目宽源码 fixture 的持续 benchmark 覆盖。

## Non-Goals

- 不消除 activity traversal，也不改变 scan/activity 两种 depth 口径。
- 不修改 `project_activity` 的 max-depth、candidate/skip pruning、symlink 或 metadata
  fallback 语义。
- 不并行化 Git lookup、candidate sizing、report assembly 或 warning aggregation。
- 不引入新线程池、新依赖、后台任务或持久化 cache。
- 不改变 candidate discovery、source/candidate bytes、rule classification、delete、
  ActionPlan、broad-root、protected-path 或任何 schema。
- 不修改依赖版本或吸收 PR #235。

## Behavior Invariants

1. **B-001** 输入为空时 activity batch 返回空结果，不启动并行工作。
2. **B-002** 输入只有一个项目时直接调用一次现有 `project_activity`，不进入 Rayon
   parallel iterator。
3. **B-003** 两个及以上项目时，每个输入目录恰好产生一个 activity 结果，使用现有
   Rayon 全局线程池，不创建无界线程或独立 pool。
4. **B-004** 输出向量与已排序输入按索引一一对应；并行完成顺序不得改变项目报告顺序。
5. **B-005** batch 内每个项目继续调用同一 `project_activity(project_dir, max_depth)`；
   nested files、depth boundary、candidate/skip pruning 和 symlink 行为不变。
6. **B-006** `project_activity` 返回 `None` 时仍以 `SystemTime::now` 保守回退；无数据不得
   产生未声明 activity source 或静默省略项目。
7. **B-007** 一个预计算值必须同时驱动 `--older-than` 判断、activity output、
   `staleness_days` 和 `risk_score`，不得在 report build 中重新遍历或重新取 activity。
8. **B-008** Git lookup、dirty-worktree safe→caution、candidate sizing、warnings、summary
   和最终 project sort 保持原有串行 materialization 语义。
9. **B-009** 同 fixture 的 before/after normalized JSON 除顶层 `scannedAt` 外完全一致；
   固定 mtime fixture 的 activity/staleness/risk 字段必须一致。
10. **B-010** 同 session 的 40,040-file fixture after median 至少比 before 快 20%；
    100-small-project Criterion point estimate 不得回退超过 10%。

## Edge Cases

- 空项目列表和单项目不承担并行调度开销。
- 某个项目在 phase 1 与 activity 阶段之间被删除时，沿用 `None → now` 的保守结果。
- activity walk 中单个 entry 消失或 metadata 失败时，沿用现有跳过该 entry 的行为。
- candidate/skip subtree 中即使有更新文件，也继续被现有 filter 排除。
- directory symlink 不跟随；symlink entry 的现有 metadata 处理不变。
- Rayon worker 数由现有全局 pool 控制；不得按项目数量创建线程。

## Boundary Checklist

| Boundary | Verdict |
| --- | --- |
| Empty / missing input | B-001、B-006 覆盖空列表和缺失项目。 |
| Error and failure paths | B-006 与 edge cases 保留现有保守 fallback。 |
| Authorization / permission | 不改变 filesystem 权限处理；沿用 `project_activity`。 |
| Concurrency / race / ordering | B-003、B-004、B-008 覆盖 bounded concurrency 与确定性。 |
| Retry / repetition / idempotency | 无 cache/持久化；每次 scan 重新计算。 |
| Illegal state transitions | B-004 保证输入/输出索引不丢失或错配。 |
| Compatibility / migration | B-007 至 B-009 保证 schema 与输出等价。 |
| Degradation / fallback | B-002、B-006 保留 direct/fallback 路径。 |
| Evidence and audit integrity | B-009、B-010 要求 normalized diff 与同 session benchmark。 |
| Cancellation / interruption / partial completion | Rayon collect 完成后才 materialize；不产生持久化半状态。 |

## Acceptance Criteria

- B-001 至 B-010 在 tech spec 和 tasks 中均有确定性映射。
- unit/regression tests 覆盖 empty、single、multiple、stable ordering、depth/pruning/symlink
  reference equivalence 和 missing-project fallback。
- before/after normalized JSON diff 为空；固定 mtime 字段一致。
- 40,040-file median 至少改善 20%，100-small benchmark 回退不超过 10%。
- full repository/MSRV/VibeGuard/three-platform gates 通过。
- Spec PR 只包含 `specs/GH284/`；implementation PR 另行创建。
