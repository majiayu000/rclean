# Interactive Restore And Read-Only Graveyard Filtering - Product Spec

## Linked Issue

- GitHub issue: `#361`
- URL: `https://github.com/majiayu000/rclean/issues/361`
- Locale: `zh-CN`
- Route: `implement`
- Complexity: `medium`

## Summary

让用户无需复制 grave ULID 即可安全恢复：无参 `rclean restore` 在真实 TTY 中列出
active graves，接受显式编号选择并在写盘前确认；`restore --dry-run` 只预览将尝试
恢复的记录；`graveyard list --older-than` 为 human/JSON 提供一致的只读年龄过滤。

每个实际恢复项仍逐个进入现有 `Graveyard::restore_by_id`。本变更不修改 overwrite、
symlink、target、cross-filesystem、manifest 或 ActionPlan 语义。

## Corrected Data Boundary

roadmap 曾规划 `graveyard list --plan <PLAN_ID>`，但当前
`ManifestRecord.plan_id` 实际保存 `ActionPlan.selected[i].id`，即每个 candidate 的
独立 ID；`ActionPlan` 没有共享 top-level plan identity。把该字段暴露成 `--plan`
会产生错误语义，因此明确排除。共享 plan identity 需要独立 schema 设计与审查。

## Goals

- 解决“先 list、再复制完整 ULID”的恢复摩擦。
- 写盘前保持“展示 → 显式选择 → 确认”三道交互。
- 多项交互恢复逐项复用既有边界检查，并完整报告部分成功。
- 为单 ID 与交互选择提供零写盘 dry-run。
- 为 graveyard list 提供确定性的删除年龄过滤。

## Non-Goals

- 不实现 `restore --since`、`restore --plan` 或 `graveyard list --plan`。
- 不实现 `--force`、自动覆盖、自动删除目标或 bulk `--to`。
- 不改变 `restore_by_id` 的 target、parent、symlink、cross-FS 或 cleanup policy。
- 不改变 manifest / ActionPlan schema、GC、retention、quota 或 grave ID。
- 不增加 TUI；本次使用跨平台编号文本交互。

## Behavior Invariants

1. **B-001** `restore --id <ID>` 必须保持现有单项恢复与 `--to <PATH>` 语义；
   `--to` 仍只允许与 `--id` 组合。
2. **B-002** 无 `--id` 的 `restore` 仅当 stdin 与 stdout 都是 terminal 时才进入
   交互；非 TTY 必须在读取选择或写盘前 fail closed，并提示使用 `--id`。
3. **B-003** 交互列表必须只包含当前 active manifest records，按
   `deleted_at` newest-first 稳定排序，并显示编号、id、删除时间、size 与
   `original_path`。
4. **B-004** 交互选择接受单号、逗号、闭区间、`a` / `all` 与 `q`；重复选择只
   恢复一次，越界/反向区间/非法输入在任何写盘前显式失败。
5. **B-005** `q` 或空选择不得恢复任何 grave，返回 exit `3`；无 active grave
   同样返回 exit `3`。
6. **B-006** 非 dry-run 的交互选择在第一次写盘前必须显示选中数量与总 size，
   并要求 `[y/N]` 确认；拒绝确认不得调用 `restore_by_id`。
7. **B-007** 每个已确认 item 必须按选择顺序单独调用现有
   `Graveyard::restore_by_id(id, None)`；不得新增 bulk move 或绕过其检查。
8. **B-008** 交互批次遇到单项错误后必须继续其余已选项，并把
   `RestoreTargetExists`、`RestoreTargetParentIsSymlink`、`GraveNotFound`
   归为 `skipped`，其他错误归为 `failed`；每项必须带 id、path 与 reason。
9. **B-009** 交互结束必须输出 restored / skipped / failed 三类计数与逐项明细。
   全部 restored 时 exit `0`；任何 skipped 或 failed 时 exit `1`，即使其他项成功。
10. **B-010** `restore --dry-run` 对 `--id` 或交互选择只输出
    “would attempt to restore” 的 id、source/target 与 size；不得移动 payload、
    rewrite manifest、创建 target parent 或执行 cleanup。
11. **B-011** dry-run 只证明“选中了哪些 records”，不声称 target preflight 或
    实际 restore 会成功；不存在的显式 `--id` 仍显式失败。
12. **B-012** `graveyard list --older-than <DURATION>` 只保留
    `now - deleted_at > duration` 的 records；未来 timestamp 不匹配；过滤后为空
    时 human 与 JSON 都返回 exit `3`。
13. **B-013** list filter 必须在 rendering 前执行一次并保留 manifest 顺序；
    human table 与 JSON 序列化同一个 filtered vector，不能出现结果分叉。
14. **B-014** `restore --since`、`restore --plan`、`graveyard list --plan`、
    `--force` 与无 `--id` 的 `--to` 必须继续被 clap 拒绝。
15. **B-015** stdout/stderr/selection/confirmation I/O 错误必须显式返回；
    不得 warning 后继续写盘或把失败伪装为成功。

## Boundary Checklist

| Boundary | Verdict |
| --- | --- |
| Empty / missing input | Covered by B-002/B-005/B-011：无 TTY、空选择、无记录、缺失 ID 均有确定结果。 |
| Error and failure paths | Covered by B-008/B-009/B-015：逐项分类、继续执行、非成功 exit。 |
| Authorization / permission | Covered by B-006：交互写盘必须确认；显式 `--id` 保持现有直接命令语义。 |
| Concurrency / race / ordering | Covered by B-003/B-007/B-008：snapshot newest-first，逐项调用；并发变化表现为 skipped/failed。 |
| Retry / repetition / idempotency | Covered by B-004/B-008：重复编号去重；并发已恢复记录显式 skipped。 |
| Illegal state transitions | Covered by B-006/B-010：未确认与 dry-run 不得进入 write transition。 |
| Compatibility / migration | Covered by B-001/B-014：现有单 ID CLI 保持，schema 不变，新歧义 flags 不存在。 |
| Degradation / fallback | Covered by B-011/B-015：dry-run 不冒充 preflight，I/O 错误不降级。 |
| Evidence and audit integrity | Covered by B-009/B-013：结果分类与 list human/JSON 绑定同一真实 records 集。 |
| Cancellation / interruption / partial completion | Covered by B-005/B-008/B-009：取消零写盘；部分完成逐项可见并 exit 1。 |

## Acceptance Criteria

- 非 TTY 无参 restore 的 CLI 回归证明 target 与 manifest 不变。
- 注入式交互测试覆盖 numbered/range/all/q、确认与非法输入。
- 两项 batch fixture（一个可恢复、一个 target conflict）证明继续执行、三分类与
  exit `1`。
- 单 ID dry-run fixture 证明 payload、manifest bytes、target parent 全部不变。
- `list --older-than` human/JSON fixtures 证明同一过滤集合与空结果 exit `3`。
- clap 负例证明 deferred/unsafe flags 不存在。
- B-001 至 B-015 在 tech spec 与 tasks 中均有确定性映射。
