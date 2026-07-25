# Scan Output Grouping (parts b, c) - Product Spec

## Linked Issue

- GitHub issue: `#358`
- URL: `https://github.com/majiayu000/rclean/issues/358`
- Locale: `zh-CN`
- Route: `implement`
- Complexity: `small`

## Summary

修复 `scan --home` human table 的两个可读性问题：当 project marker 不存在时，
`Kind` 应显示 candidate 已有 `rule_id` 的生态前缀，而不是整列 `Unknown`；同一
app/parent 下的候选应稳定按大小排列并形成视觉分组，同时保留“一行 = 一个可删
candidate”的现有契约。

Issue #358 的 part (a) 已由 PR #359 修复。本 spec 只覆盖 maintainer 已选择的
B-b1 + B-c1。真正把多个 candidates 合成一条可选择父行的 B-c2 不在本次范围内。

## Problem

`detect_project_kind` 依赖 project marker。全局缓存通常没有 marker，所以
`scan --home` 的 `Kind` 列几乎全是 `Unknown`，即使 candidate 的 `rule_id`
已经明确包含 `go`、`node`、`cargo`、`editor` 等生态。

同一 app/parent 的 candidates 当前虽然在数据结构中属于同一 `ProjectReport`，
但 human table 重复打印 project path，且沿用扫描输入顺序。多个
`Cache` / `CachedData` / `GPUCache` / `logs` 行视觉上分散，主要回收项也可能
埋在小项之后。

## Goals

- 已知 project kind 保持现有展示。
- `Unknown` project kind 使用 candidate 的 `rule_id` 生态前缀。
- 同一 project 内按 candidate size 稳定降序。
- human table 用 project 首行 + continuation rows 表达视觉分组。
- 每个 candidate 仍独立显示，JSON、selection 和 ActionPlan 完全不变。

## Non-Goals

- 不把多个 candidates 折叠为 `"N items"` 汇总行。
- 不新增父级 selection，不改变 candidate identity 或选择映射。
- 不改变 `ScanReport` / ActionPlan schema、JSON 顺序或字段。
- 不改变 safety、risk、staleness、size、reason 或 classification。
- 不改变 `Biggest wins` 的既有排序。

## Behavior Invariants

1. **B-001** 当 `project.kind != "Unknown"` 时，human table 的 `Kind` 必须继续
   显示原 project kind，不得被 `rule_id` 前缀覆盖。
2. **B-002** 当 `project.kind == "Unknown"` 且 `rule_id` 含 `.` 时，`Kind`
   必须显示第一个 `.` 之前的非空生态前缀，例如
   `go.build_cache -> go`、`editor.vscode_cache -> editor`。
3. **B-003** 当 `project.kind == "Unknown"` 且 `rule_id` 不含 `.` 时，`Kind`
   必须显示完整 `rule_id`；展示层不得制造另一个 `Unknown` fallback。
4. **B-004** human table 必须为每个 `Candidate` 保留一条独立数据行；不得折叠、
   汇总、隐藏或用父行替代 candidate。
5. **B-005** 同一 `ProjectReport` 内的数据行必须按 `bytes` 稳定降序；相同
   `bytes` 的 candidates 保持原输入顺序，确保输出确定性。
6. **B-006** 同一 project 的第一条 candidate 行显示 project path，后续
   continuation rows 的 Project 单元格留空；新 project 的第一行重新显示自己的
   path。该视觉分组不得改变其他列的 candidate 值。
7. **B-007** `scan --json` 必须保持原 candidate 数量、顺序、字段和值；human
   table 的派生 Kind 和排序不得进入 JSON。
8. **B-008** `clean`、`tui`、`free`、ActionPlan write/replay 的 candidate identity、
   selection、顺序和安全语义保持不变；它们不得读取 human table render plan。
9. **B-009** stdout 写入失败仍通过现有 `RcleanError` 路径显式返回；视觉分组不得
   通过吞掉输出错误来伪装成功。

## Edge Cases

- project 没有 candidates：保持现有行为，不产生空 continuation row。
- project 只有一个 candidate：正常显示 project path，不增加空分隔数据行。
- `rule_id` 没有点号：显示完整值。
- candidates 大小相同：保持输入顺序。
- 多个 projects：每个 project 都独立开始自己的视觉分组。

## Boundary Checklist

| Boundary | Verdict |
| --- | --- |
| Empty / missing input | Covered by B-004/B-006：空 project 不制造行，单 candidate 正常显示。 |
| Error and failure paths | Covered by B-009：保持 stdout 错误显式失败。 |
| Authorization / permission | N/A：human scan output 不写盘、不改变授权。 |
| Concurrency / race / ordering | Covered by B-005：稳定排序锁定确定性。 |
| Retry / repetition / idempotency | Covered by B-005/B-007：同一 report 重绘结果稳定，JSON 不变。 |
| Illegal state transitions | N/A：没有状态机或持久化转换。 |
| Compatibility / migration | Covered by B-007/B-008：JSON、ActionPlan 与 selection 契约不变。 |
| Degradation / fallback | Covered by B-002/B-003/B-009：Kind 有确定派生规则，错误不静默降级。 |
| Evidence and audit integrity | Covered by B-004/B-007：human 与 JSON candidate 计数可逐项核对。 |
| Cancellation / interruption / partial completion | Covered by B-009：沿用现有输出失败语义；不新增写盘副作用。 |

## Acceptance Criteria

- `scan --home` 的全局 cache 行显示生态 Kind，不再整列 `Unknown`。
- 已知 Rust / Node.js 等 project kind 保持不变。
- 多 candidate project 的 human table 按大小稳定降序，project path 只在组首行显示。
- human table 中 candidate 数据行数与 report candidate 数相同，不出现汇总行。
- `scan --json` fixture 在改动前后结构与 candidate 顺序一致。
- B-001 至 B-009 在 tech spec 与 tasks 中均有确定性验证映射。
