# Project-Scoped Risk Score Reuse - Product Spec

## Linked Issue

- GitHub issue: `#296`
- URL: `https://github.com/majiayu000/rclean/issues/296`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

扫描同一个项目的多个候选时，惰性计算一次项目级 `risk_score` 并复用，避免每个候选重复
探测同一组 lockfile。保持 risk 公式、候选过滤、顺序、输出 schema 和所有安全/删除行为不变。

## Problem

`src/scan/project.rs::build_project_report` 在 candidate 循环内调用
`compute_risk_score`。同一项目的 `git`、`activity_time` 和 project path 完全相同，但该函数
每次都会通过 `has_lockfile` 最多检查 12 个 lockfile 路径。

最新 `origin/main@b8116e7` 的固定 fixture 含 1,000 个 Node 项目，每项目 8 个可报告候选，
总计 8,000 个候选。在没有 lockfile 时，当前调用图约执行 96,000 次文件探测；项目级复用
只需要约 12,000 次，减少 84,000 次（87.5%）。关闭 Git 的 warmed release 基线 15 次
范围为 0.50–3.08s，median 1.05s；主机噪声较高，因此最终性能结论必须采用同 session
交错 before/after 测量，而不是只比较两个孤立样本。

## Goals

- 保留现有过滤顺序：只有候选通过当前 min-size/safety 条件后才触发 risk 计算。
- 一个项目没有候选通过过滤时不计算 risk；有一个或多个时最多计算一次。
- 同一 `ProjectReport` 中所有候选复用相同的项目级 risk 值。
- 保留 standalone `compute_risk_score` 与 `explain` 路径行为。
- 增加固定多候选项目 benchmark shape，持续覆盖该热点。
- 用 normalized JSON 和现有测试证明报告、安全与选择语义不变。

## Non-Goals

- 不改变 risk 公式、权重、阈值、root-boundary deferred 语义或 JSON 字段。
- 不缓存跨项目、跨 root 或跨 scan 状态，不持久化 lockfile 结论。
- 不改变 marker/rule classification、git/activity 计算、candidate sizing 或排序。
- 不改变 safety tier、clean selection、ActionPlan、delete、graveyard、symlink、broad-root 或
  protected-path policy。
- 不新增 CLI flag、配置、依赖、schema 或 wall-clock CI assertion。
- 不修改私有 security advisory 或吸收 Dependabot PR #235。

## Behavior Invariants

1. **B-001** 所有候选都被现有过滤条件排除时，项目 risk 初始化闭包调用 0 次。
2. **B-002** 至少一个候选通过过滤时，项目 risk 初始化闭包调用恰好 1 次，并被所有后续
   included candidates 复用。
3. **B-003** risk 仍由相同的 `git`、`activity_time`、project path、lockfile 集与既有公式
   计算；不得缓存跨 project/scan 结果。
4. **B-004** public/internal standalone `compute_risk_score` 调用和 `explain` 继续独立读取
   当前项目 lockfile 状态，返回契约不变。
5. **B-005** min-size、blocked/report-only 例外、candidate 顺序、bytes、warnings、requires
   sudo、staleness、summary 和排序完全不变。
6. **B-006** 固定 fixture 的 baseline/implementation normalized scan JSON 除顶层
   `scannedAt` 外为空 diff。
7. **B-007** durable benchmark 包含每项目多个候选，fixture 构造在 timed closure 外，
   不添加时间断言。
8. **B-008** 1,000×8 no-lockfile fixture 的静态 probe 上界从约 96,000 降为约 12,000；
   同 session 交错 release median 至少改善 15%，现有 benchmark shapes 不回退超过 10%。
9. **B-009** implementation diff 仅修改 `src/scan/project.rs` 与
   `benches/scan_throughput.rs`，并通过 stable/MSRV/VibeGuard/三平台 CI/SpecRail 门。

## Edge Cases

- 一个项目只有一个 included candidate：只计算一次，输出与当前相同。
- 前几个候选因 min-size 被排除，后续候选被保留：在首个保留候选处惰性计算一次。
- 全部 safe/caution 候选低于 min-size：不做 lockfile 探测。
- blocked/report-only 候选继续遵守当前 min-size 例外，若被报告则可触发首次计算。
- lockfile 存在：仍使用当前 12 项静态顺序和 `Path::is_file` short-circuit 语义。
- 无 lockfile：完整探测一次，不为每个候选重复探测。
- activity 正好跨 7 天边界：同一报告使用一个一致项目级值，不再允许候选间因循环耗时
  出现微小差异。

## Acceptance Criteria

- B-001 至 B-009 在 tech spec/tasks 中有完整映射。
- focused test 以计数闭包证明 lazy cache 的 zero/once/reuse 契约。
- 既有 risk-score、scan 和 explain tests 全部通过。
- normalized baseline/implementation JSON 除 `scannedAt` 外无差异。
- 多候选 benchmark 与 15 次交错 release 测量满足 B-008。
- Spec PR 与 implementation PR 分离；实现始于合并后的最新 `origin/main`。
- full stable/release/MSRV/VibeGuard/CI/PR gates 通过。
