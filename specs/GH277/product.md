# Free Machine-Readable Proposal Output - Product Spec

## Linked Issue

- GitHub issue: `#277`
- URL: `https://github.com/majiayu000/rclean/issues/277`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `medium`

## Summary

兑现 `rclean free <size> --json` 的 CLI 契约：非交互 free 的 stdout 必须是单个、带版本的
JSON proposal，覆盖目标是否满足、选中总量、候选详情和已写入的 ActionPlan 路径，同时保留
现有人类输出、选择排序、退出码和安全边界。

## Problem

`CommonScanArgs` 和 `free --help` 已公开 `--json`，但 `free::run` 只有 interactive conflict
检查，没有普通 JSON 输出分支。成功命令会在 stdout 打印 `Proposed set`、candidate、total、
plan path 和 review hint；`jq` 因此解析失败，即使 free 自身退出 0。

这不是缺少一个隐藏功能，而是公开 flag 与真实行为不一致。机器调用者无法稳定读取 proposal，
也无法区分 target-met、shortfall 和 no-candidate 结果。

## Goals

- 普通 `free --json` stdout 只包含一个有效 JSON document。
- schema 自带版本，并用 camelCase 字段。
- target met、target shortfall、no candidates 三种结果均有确定 JSON。
- candidate 复用 scan `Candidate` 的现有 JSON 字段，不维护第二套候选 schema。
- 有候选时仍先成功写 ActionPlan，再输出包含 plan path 的 JSON。
- 没有候选时不写 ActionPlan，JSON 中 `planPath` 为 `null`。
- 保留 target met 的 exit 0 与 shortfall/no-candidate 的 exit 3。
- 不带 `--json` 时的人类输出逐句保持现状。

## Non-Goals

- 不改变 safe-only eligibility、staleness/risk/size 排序或 reverse prune。
- 不改变 ActionPlan schema、内容、默认文件名、delete mode 或 replay。
- 不让 `free --interactive --json` 合法；该组合继续明确拒绝。
- 不改变 scan JSON、scan warnings、progress reporter 或 stderr 策略。
- 不实现流式 JSON、JSON Lines 或新的 output file flag。
- 不处理通用 stdout broken-pipe 行为；该问题独立审计。
- 不改变 clean、删除、graveyard、symlink、broad-root 或 protected-path 行为。

## JSON Contract

顶层对象字段：

| Field | Type | Meaning |
| --- | --- | --- |
| `schemaVersion` | integer | 当前固定为 `1`。 |
| `targetBytes` | integer | 用户请求释放的精确 byte 数。 |
| `selectedBytes` | integer | safe proposal 候选 byte 总和。 |
| `targetMet` | boolean | 存在非空 safe proposal，且 `selectedBytes >= targetBytes`。 |
| `planPath` | string or null | 成功写入的 ActionPlan path；无候选、未写计划时为 `null`。 |
| `candidates` | array | 按实际 proposal 排名顺序排列的现有 `Candidate` JSON objects。 |

每个 candidate 复用当前 scan contract：`path`、`name`、`ruleId`、`category`、`bytes`、
`safety`、`requiresSudo`、`reasons`、`warnings`、`restoreHint`、`riskScore`，以及存在时的
`stalenessDays`。不得复制一个字段子集后让两个 schema 独立演化。

## Behavior Invariants

1. **B-001** 普通 `free --json` stdout 是单个 JSON document，不包含 proposal header、格式化
   size、plan hint、gap hint、progress 或其他人类文本。
2. **B-002** 顶层 schema 精确包含 contract 表中的六个字段，使用 camelCase 且
   `schemaVersion` 为 `1`。
3. **B-003** `candidates` 复用 `Candidate` serialization 并保持 proposal 顺序；每项具备
   rule/category/safety/bytes/sudo/risk/staleness 等机器决策字段。
4. **B-004** target met 时先写 ActionPlan，再输出 `targetMet=true`、非空 `planPath`，exit 0。
5. **B-005** safe proposal 不足时仍写 reviewable ActionPlan，输出 `targetMet=false` 与非空
   `planPath`，exit 3；不输出人类 gap 文本。
6. **B-006** 没有 eligible candidate 时输出 empty `candidates`、`selectedBytes=0`、
   `targetMet=false`、`planPath=null`，不写计划，exit 3；`targetBytes=0` 也保持这个现有
   no-candidate 结果，不把数学上的 `0 >= 0` 误报成命令成功。
7. **B-007** plan serialization/write 或 proposal JSON serialization 失败时返回现有 error，
   不在 stdout 留下半个 JSON 或前置人类文本。
8. **B-008** `free --interactive --json` 继续在任何 cleanup 前失败，错误在 stderr。
9. **B-009** 不带 `--json` 的 target met、shortfall、no-candidate 和 interactive 输出/退出码
   保持现有行为。
10. **B-010** 实现不改变选择、安全、删除或 ActionPlan schema；focused、stable、release、
    exact MSRV、VibeGuard 与 current-head PR gates 全部通过。

## Acceptance Criteria

- B-001 至 B-010 在 tech spec 和 tasks 中完整映射。
- E2E tests 对三种 JSON 结果执行 `serde_json::from_slice`，并断言 stdout 无人类提示污染。
- target-met 与 shortfall tests 同时解析实际 ActionPlan，证明 JSON path 指向已写入计划。
- no-candidate tests 覆盖正数和 zero target，并证明未创建默认或显式 plan。
- 现有人类输出 tests 保持通过。
- README 给出 `free --json` 用法，architecture 文档记录独立 versioned schema。
- Spec PR 只包含 `specs/GH277/`；implementation PR 独立创建。
