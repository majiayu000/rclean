# Extract Watch Tests Into A Dedicated Module - Product Spec

## Linked Issue

- GitHub issue: `#317`
- URL: `https://github.com/majiayu000/rclean/issues/317`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

把 `src/watch/mod.rs` 中 inline `#[cfg(test)]` block 机械搬移到 `src/watch/tests.rs`。父模块只保留
`#[cfg(test)] mod tests;`，production watch implementation、九个测试、两个 helpers、fixtures 与
assertions 全部不变。拆分并按 crate edition 2024 运行 rustfmt 后，父文件为 320 行，child 为
229 行，两者都回到仓库 200–400 行的典型范围。

## Problem

最新 `origin/main@581b475` 的 `src/watch/mod.rs` 共 553 行：production 结束于第 318 行，第
319–553 行是 inline test module，其中 body 为第 321–552 行、共 232 行。plan-path collision、
probe errors、lockfile mapping 与 refresh reconciliation tests 和 runtime watch code 共享一个 553 行
review surface。

历史 GH274/GH293 修改过 watch behavior/tests，但没有覆盖 test-module extraction。GitHub issues/PRs、
`docs/specs/` 与 `specs/` 搜索没有发现同范围工作。仓库已有多处 `module/tests.rs` 组织模式。

## Goals

- 让 watch production module 和 test-only child 各自落入典型行数范围。
- 用 crate-edition-2024 normalized exact proof 保证纯搬移。
- 保留九个测试、`candidate_map`、`report_with_projects` 与 Unix-only non-UTF-8 test cfg。
- 缩小后续 watch runtime review surface。

## Non-Goals

- 不重写、重命名、增加、删除或合并任何测试/helper/assertion/fixture。
- 不修改 ActionPlan naming、collision probing、error propagation、lockfile mapping、refresh scope、
  reconciliation、path boundary、watch output 或 runtime behavior。
- 不改变 visibility、API、CLI、JSON、依赖、feature、workflow、docs 或 trust-model policy。
- 不顺带拆分其他大文件。

## Behavior Invariants

1. **B-001** `src/watch/mod.rs` 基线第 1–319 行完全不变，第 320 行精确为 `mod tests;`；最终
   parent 恰好 320 行。
2. **B-002** `src/watch/tests.rs` 等于基线第 321–552 行去掉一层四空格后，经当前 toolchain
   `rustfmt --edition 2024` 规范化的结果；最终 child 恰好 229 行。
3. **B-003** 九个 test names、`candidate_map`、`report_with_projects`、imports、fixtures、cfg 与
   assertions 保持不变。
4. **B-004** numeric suffix、extensionless/non-UTF-8 names、probe/sequence errors、lockfile mapping、
   missing/empty/single-project refresh reconciliation contracts 保持不变。
5. **B-005** implementation diff 只允许修改 `src/watch/mod.rs` 并新增 `src/watch/tests.rs`。
6. **B-006** focused/full stable/release、精确 Rust 1.95.0、VibeGuard、SpecRail、独立 review 与
   current-head cross-platform CI/PR gates 全绿。

## Edge Cases

- child 的 `use super::*` 继续访问 parent 私有项，不扩大 visibility。
- 只搬移 outer wrapper body；`#[cfg(test)]` 保留在 parent declaration。
- Unix-only non-UTF-8 test 的 cfg 与 path-byte semantics 必须原样保留。
- body 不含 `file!`、`line!`、`column!`、`module_path!`、`include!` 或 `#[path]`；实现复核再次扫描。
- 若 implementation base 的 watch layout 漂移，停止并刷新 proof，不套用旧坐标。
- exact proof 在 fmt 后运行，dedented baseline 必须经与 `Cargo.toml` 相同的 edition 2024 rustfmt。

## Acceptance Criteria

- B-001..B-006 在 tech/tasks 完整映射。
- Spec 与 implementation PR 分离；实现从 Spec 合并后的最新 main 开始。
- 320/229 line-count、exact forward proof 与 reverse rollback proof 通过 stable/MSRV。
- 九个 focused tests 和 full gates 通过，无 test weakening。
- current-head CI、签名、reviewThreads、merge state、独立 review 与 SpecRail gate 全绿后才合并。
