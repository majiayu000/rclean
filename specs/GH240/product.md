# Candidate Size Warning Propagation - Product Spec

## Linked Issue

- GitHub issue: `#240`
- URL: `https://github.com/majiayu000/rclean/issues/240`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `medium`

## Summary

当候选目录在尺寸计算阶段无法完整读取时，保留已经计算出的部分尺寸，同时把
失败记录到现有 `ScanReport.warnings`。用户和 JSON consumers 必须能够区分
“完整的 0 bytes”与“因读取失败得到的 0 或部分结果”。

## Problem

phase 1 walker 会剪枝已分类候选，候选内部文件由 phase 2 sizer 单独遍历。
当前 sizer 的 metadata、`read_dir` 和 walk 错误仅进入 `debug!`，并返回 `0`
或部分累计值。最终报告仍包含 `warnings: []`，人类输出也不会显示
“Results may be incomplete”。这使错误结果看起来像成功结果。

## Goals

- 让候选尺寸阶段的每个可恢复读取错误进入结构化 scan warning。
- 出错时保留安全可得的部分尺寸，不把整个扫描升级为 fatal error。
- 串行与并行 sizing 路径采用相同的错误可见性语义。
- 保持 warning 输出确定，避免并行调度改变报告顺序。
- 不改变分类、过滤、安全等级、选择或删除契约。

## Non-Goals

- 不重试权限失败，也不提升权限或调用 `sudo`。
- 不把可恢复的单候选读取错误改为整个 scan 失败。
- 不估算、猜测或补偿无法读取的 bytes。
- 不修改已有 `ScanWarning` JSON 字段、kind 名称或 schema version。
- 不改变 phase 1 walker 已有 warning 行为或进行无关性能重构。

## Behavior Invariants

1. **B-001** 当候选根路径或其后代发生 walk、`read_dir` 或 metadata 读取失败
   时，最终 `ScanReport.warnings` 必须至少包含一个描述实际失败路径和错误的
   `WalkError` 或 `MetadataError`；不得只写 debug 日志。
2. **B-002** sizing 失败是可恢复降级：扫描必须继续，并返回失败发生前或其他
   可读分支已经累计的准确 bytes；不得把未知 bytes 当作已读取，也不得把已有
   部分累计值清零。
3. **B-003** 完全可读候选的 bytes、warning 集合和排序结果必须与修复前保持
   一致，不得为成功读取生成 warning。
4. **B-004** 小目录串行 partition、宽目录 parallel walker、多 root rayon walk
   和候选根 metadata 四条 sizing 路径必须遵守 B-001 至 B-003 的同一语义。
5. **B-005** 新增的 sizing warnings 必须使用稳定顺序合并；相同目录树重复扫描
   不得因并行 worker 调度产生不同 warning 顺序。
6. **B-006** JSON 输出必须在顶层 `warnings` 中序列化 sizing warning；人类输出
   必须复用现有 warning summary，并明确结果可能不完整。
7. **B-007** 当 `--min-size`、排序或 `free <size>` 消费部分 bytes 时，必须使用
   实际累计值并同时保留 warning；不得用 warning 绕过现有过滤或选择规则。
8. **B-008** 修复不得改变候选分类、安全等级、blocked/report-only 处理、
   ActionPlan revalidation、clean 选择或删除行为。

## Edge Cases

- 候选根在分类后、sizing 前消失：返回 `0` 并记录根路径 metadata warning。
- 单链目录中途不可读：保留此前同级文件 bytes，并记录不可读目录。
- 多分支候选只有一个分支失败：累计其他分支 bytes，扫描继续。
- parallel walker 同时报告多个错误：每个实际失败可见，最终顺序稳定。
- blocked 候选仍不执行 sizing；不得因为本变更为未读取的 blocked 路径生成新
  warning。

## Boundary Checklist

| Boundary | Verdict |
| --- | --- |
| Empty / missing input | Covered by B-001/B-002：候选消失时返回 0 并记录根 metadata warning。 |
| Error and failure paths | Covered by B-001、B-002、B-004。 |
| Authorization / permission | Covered by B-001/B-002：权限拒绝可见但不提权。 |
| Concurrency / race / ordering | Covered by B-004/B-005：并行路径与确定性合并。 |
| Retry / repetition / idempotency | Covered by B-005：重复扫描在相同文件状态下顺序稳定。 |
| Illegal state transitions | N/A：sizing 不维护状态机。 |
| Compatibility / migration | Covered by B-003/B-006/B-008：复用现有 warning schema，无迁移。 |
| Degradation / fallback | Covered by B-001/B-002/B-007：部分结果不得伪装为完整成功。 |
| Evidence and audit integrity | Covered by B-006：机器和人类输出都保留失败证据。 |
| Cancellation / interruption / partial completion | Covered by B-002：单路径失败产生显式部分结果；scan 进程取消仍沿用现有行为。 |

## Acceptance Criteria

- B-001 至 B-008 在 tech spec 和 tasks 中均有明确验证映射。
- 真实不可读候选复现从 `warnings: []` 变为非空结构化 warning。
- 正常候选 sizing parity 测试继续通过。
- 并行错误 warning 顺序有确定性测试。
- Spec PR 只包含 `specs/GH240/`；implementation PR 另行创建。
