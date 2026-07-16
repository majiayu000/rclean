# GitCache Test Module Split - Product Spec

## Linked Issue

- GitHub issue: `#262`
- URL: `https://github.com/majiayu000/rclean/issues/262`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

把 `src/scan/git_cache.rs` 的内联测试模块原样迁移到
`src/scan/git_cache/tests.rs`，在不改变任何生产行为或测试语义的前提下，为接近
800 行硬上限的生产文件恢复演进空间。

## Problem

`origin/main@8616f50` 上的 `src/scan/git_cache.rs` 为 743 行，距离仓库 800 行硬上限
仅 57 行。其中生产实现约 413 行，剩余约 328 行是边界清晰的
`#[cfg(test)] mod tests`。继续内联增加任何 Git 回归用例都会迫使后续无关改动同时完成
拆分，扩大 review scope。

## Goals

- 生产模块只保留实现和外部测试模块声明，目标不超过 450 行。
- 12 个既有 GitCache 测试的名称、断言、helper、fixture 和平台条件完全不变。
- 测试继续以 child module 访问 parent module 的私有实现，不扩大可见性。
- 两个结果文件均低于 800 行硬上限。
- diff 保持机械、可审计，不混入行为或格式“顺手优化”。

## Non-Goals

- 不改变 Git discovery、marker cache、dirty-state、timeout 或 poison 语义。
- 不重命名、合并、删除、压缩或新增测试。
- 不改变 API、类型可见性、CLI、JSON、warning、safety classification 或 ActionPlan。
- 不拆分生产类型/函数，不新增依赖，不修改其他测试或文档。
- 不吸收 Dependabot PR #235。

## Behavior Invariants

1. **B-001** `src/scan/git_cache.rs` 的生产 token 内容保持不变，只把内联 test module
   替换为 `#[cfg(test)] mod tests;`。
2. **B-002** `src/scan/git_cache/tests.rs` 包含迁移前 test module body 的完整等价内容；
   12 个 `scan::git_cache::tests::*` 测试名称不变。
3. **B-003** test module 继续通过 `use super::*` 访问 parent private items；不得为了拆分
   提高任何生产 item 的可见性。
4. **B-004** 所有断言、fake runner、metadata probe、temp repo fixture、poison fixture、
   timeout helper 和 Unix/Windows conditional 保持不变。
5. **B-005** `src/scan/git_cache.rs` ≤450 行，`src/scan/git_cache/tests.rs` <800 行。
6. **B-006** implementation diff 只允许上述两个路径；Cargo/dependency/API/output/safety
   行为零变化。
7. **B-007** focused test identity、full stable/MSRV/VibeGuard/three-platform gates 全部通过。

## Acceptance Criteria

- B-001 至 B-007 在 tech spec 和 tasks 中均有确定验证映射。
- 迁移前后 `cargo test scan::git_cache::tests -- --list` 的排序后 test-name set 完全相同，
  且均为 12 个。
- 生产代码片段做忽略 test-module declaration 的机械比较时无差异。
- 行数和 implementation scope 满足 B-005/B-006。
- Spec PR 只包含 `specs/GH262/`；implementation PR 独立创建。
