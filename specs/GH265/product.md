# Doctor Global Catalog Coverage - Product Spec

## Linked Issue

- GitHub issue: `#265`
- URL: `https://github.com/majiayu000/rclean/issues/265`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

把 `doctor` 的 global-rule 回归保护从脆弱的固定数量断言升级为与现有 rule catalog 的
精确 ID 集合对照，并拒绝重复 doctor ID。同时把内联测试迁移到 child module，为
744 行的生产文件恢复空间；运行时行为保持不变。

## Problem

当前 `diagnose_returns_one_entry_per_phase1_global_rule` 只断言
`report.total_count() == 59`。如果一个 global rule 被遗漏、另一个被重复或替换，而总数
仍为 59，测试会静默通过。fresh static evidence 证明当前 59 个 catalog global IDs
与 doctor 一致，缺口是缺少能长期保持“身份一致”的回归断言。

`src/doctor.rs` 同时已达 744 行，测试模块从第 652 行开始。直接增加更强测试会进一步
逼近 800 行硬上限。

## Goals

- doctor 默认报告的 rule ID 集合必须精确等于现有 catalog 中 `is_global_rule` 的集合。
- doctor 默认报告不得包含重复 rule ID。
- 移除固定 `59` magic number，使新增/删除 global rule 的 drift 由身份差异解释。
- 测试迁移到 `src/doctor/tests.rs`，生产内容保持不变且父文件 <700 行。
- 既有 applicable/skipped 行为测试保持不变。

## Non-Goals

- 不修改 doctor entry、顺序、输出、Docker opt-in 或平台分支。
- 不新增生产 registry，不改变 catalog、`is_global_rule` 或任何 rule classifier。
- 不改变 API/可见性、依赖、scan、clean、safety 或 ActionPlan。
- 不把非-global project rule 加入 doctor。
- 不吸收 Dependabot PR #235。

## Behavior Invariants

1. **B-001** `src/doctor.rs` 的生产内容逐行不变，只把内联 tests block 替换为
   `#[cfg(test)] mod tests;`，父文件 <700 行。
2. **B-002** 新的 catalog coverage test 从 `crate::rules::rule_catalog()` 过滤
   `crate::rules::is_global_rule` 得到 expected set，与 `diagnose()` 的 actual set 精确相等。
3. **B-003** coverage test 单独断言 `report.entries.len() == actual_set.len()`，重复 doctor
   rule ID 必须失败。
4. **B-004** 不再断言硬编码总数 `59`；集合差异必须输出可诊断的 missing/extra IDs。
5. **B-005** `diagnose_marks_existing_anchor_applicable`、
   `diagnose_marks_missing_anchor_skipped` 及其 HOME guard 语义保持不变。
6. **B-006** implementation scope 只允许 `src/doctor.rs` 与 `src/doctor/tests.rs`，无生产
   API、registry、dependency、output 或 safety 变化。
7. **B-007** focused doctor、full stable/MSRV/VibeGuard/three-platform gates 全部通过。

## Acceptance Criteria

- B-001 至 B-007 在 tech spec 和 tasks 中完整映射。
- focused tests 共 3 个：精确 catalog coverage、existing anchor、missing anchor。
- production prefix 逐行比较无差异，两个既有状态测试 body 比较无差异。
- parent <700 行、child <800 行、scope 精确为两个 manifest 路径。
- Spec PR 只包含 `specs/GH265/`，implementation PR 独立创建。
