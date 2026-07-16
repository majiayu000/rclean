# Canonical Scan Root Deduplication - Product Spec

## Linked Issue

- GitHub issue: `#253`
- URL: `https://github.com/majiayu000/rclean/issues/253`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

当多个 scan root canonicalize 到同一目录时，rclean 必须只扫描该目录一次，避免
重复项目、候选、回收量和 ActionPlan 删除项。去重只覆盖完全相同的 canonical
root，不合并具有不同扫描语义的祖先与后代 roots。

## Problem

当前 `scan()` 在处理每个输入时先 canonicalize，然后立即把结果加入报告并执行
walker。seen set 不存在，因此相同路径重复传入，或 `path` 与 `path/.` 等不同拼写
解析到同一目录时，同一 candidate 会被多次报告并写入 ActionPlan。replay 随后会
对同一路径安排重复操作，汇总数据也不再可信。

## Goals

- 在文件遍历、git 探测、sizing 和 report 构建前去重完全相同的 canonical roots。
- 保留每个 canonical root 第一次出现的输入顺序。
- 让 scan 的 human/JSON 输出和 ActionPlan 对同一 canonical root 只计一次。
- 保持错误、安全、分类和删除契约不变。

## Non-Goals

- 不合并祖先/后代 roots，也不推断重叠目录的等价性。
- 不改变 `max-depth`、`.rcleanignore`、user rules 或 root-specific config 语义。
- 不改变 candidate classification、safety tier、ActionPlan schema 或 clean revalidation。
- 不顺带重构 walker、git cache、sizer 或 plan writer。

## Behavior Invariants

1. **B-001** 两个或更多输入 canonicalize 到同一绝对目录时，该 canonical root
   必须只进入扫描 pipeline 一次。
2. **B-002** 去重后 root 顺序必须等于各 canonical root 第一次出现的顺序；后续
   重复输入不得重排已有 roots。
3. **B-003** 字符串不同但 canonicalize 结果相同的输入（例如 `path` 与
   `path/.`）必须与字面重复路径采用相同去重语义。
4. **B-004** 去重必须发生在 walker、git、sizing、project materialization 和
   summary 之前；不得先重复工作再只隐藏最终输出。
5. **B-005** human/JSON summary、`roots`、projects、candidates 和新写入的
   ActionPlan 对每个 exact canonical root 只计一次；plan dry-run 不得包含重复路径。
6. **B-006** canonicalize 结果不同的 roots 必须保留，即使它们是祖先/后代或目录
   内容重叠；本变更不得扩大等价关系。
7. **B-007** 任一唯一输入无法 canonicalize 时，scan 继续返回现有
   `CanonicalizeRoot` 错误并指向原始输入；不得把失败输入当作重复项静默丢弃。
8. **B-008** 单 root 和多 unique-root scan 的分类、safety、warning、排序、
   ActionPlan schema 与 clean/delete 行为必须保持不变。

## Edge Cases

- 空 paths slice：保持现有空报告语义；CLI 仍由上层补当前目录。
- 同一路径连续或非连续重复：均只保留第一次。
- symlink/`.`/`..` 拼写解析到同一已存在目录：按 canonical 结果去重。
- ancestor + descendant：两者 canonical 路径不同，必须都保留。
- 一个有效 root 加一个不存在 root：仍返回不存在 root 的 canonicalize 错误，不返回
  部分成功报告。

## Boundary Checklist

| Boundary | Verdict |
| --- | --- |
| Empty / missing input | Covered by B-008：保持现有空 slice/CLI 默认行为。 |
| Error and failure paths | Covered by B-007：canonicalize 失败不得被静默降级。 |
| Authorization / permission | N/A：scan root 去重不改变权限或授权。 |
| Concurrency / race / ordering | Covered by B-002/B-004：去重先于并行 walker 且保留首个顺序。 |
| Retry / repetition / idempotency | Covered by B-001/B-003：重复输入是幂等的。 |
| Illegal state transitions | N/A：该流程不维护状态机。 |
| Compatibility / migration | Covered by B-005/B-008：schema 不变，仅删除错误重复项。 |
| Degradation / fallback | Covered by B-007：失败不得伪装为成功去重。 |
| Evidence and audit integrity | Covered by B-004/B-005：内部工作量与所有输出层都必须唯一。 |
| Cancellation / interruption / partial completion | Covered by B-007：错误时不产生部分成功报告；其余取消行为不变。 |

## Acceptance Criteria

- 修复前能稳定复现的重复-root CLI/ActionPlan 测试在修复后通过。
- `path path` 与 `path path/.` 都只生成一个 root、project、candidate 和 selected item。
- ancestor + descendant 负例证明不同 canonical roots 未被合并。
- B-001 至 B-008 在 tech spec 与 tasks 中均有确定性验证映射。
- Spec PR 只包含 `specs/GH253/`；implementation 另从 Spec 合并后的最新
  `origin/main` 创建。
