# Extract Project Rule Tests Into A Dedicated Module - Product Spec

## Linked Issue

- GitHub issue: `#314`
- URL: `https://github.com/majiayu000/rclean/issues/314`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

把 `src/rules/project.rs` 中 248 行 inline `#[cfg(test)]` block 机械搬移到
`src/rules/project/tests.rs`。父模块只保留 `#[cfg(test)] mod tests;`，production classifier、九个
测试、helpers、fixtures、assertions 与跨平台 symlink 语义全部不变。拆分后父文件为 384 行，
test-only child 为 245 行，两者都回到仓库 200–400 行的典型范围。

## Problem

最新 `origin/main@9a79ade` 的 `src/rules/project.rs` 共 630 行：production classifier 结束于
第 382 行；第 383–630 行是 inline test module，其中 body 为第 385–629 行、共 245 行。规则
实现和覆盖其 marker order、kind priority、snapshot fallback、non-UTF-8、symlink 与 candidate
prefilter contracts 的测试共享一个 630 行 review surface。

仓库已有 `src/scan/git_cache.rs` + `src/scan/git_cache/tests.rs`、`src/doctor.rs` +
`src/doctor/tests.rs`、`src/clean/deletion.rs` + `src/clean/deletion/tests.rs` 的相同组织模式。GitHub
issues/PRs、`docs/specs/` 与 `specs/` 搜索没有发现已覆盖本次 test-only extraction 的工作。

## Goals

- 让 project rule implementation 和 test-only module 各自落入典型行数范围。
- 用 rustfmt-normalized exact relocation proof 保证没有手工测试或 production 改动。
- 保留九个测试、两个 helpers、`ProjectKindCase` alias 及 Unix/Windows symlink helpers。
- 缩小后续 project classifier review 的噪声范围。

## Non-Goals

- 不重写、合并、重命名、增加或删除测试/helper/assertion/fixture。
- 不修改 candidate names、marker matching/order、kind priority、snapshot limit/fallback、non-UTF-8、
  symlink、global app cache prefilter 或任何 classifier 行为。
- 不改变 visibility、API、CLI、JSON、依赖、feature、workflow、README 或 trust-model policy。
- 不顺带拆分 `scan/project.rs`、其他 rule module 或其他 400–800 行文件。

## Behavior Invariants

1. **B-001** `src/rules/project.rs` 基线第 1–383 行必须完全不变，并在第 384 行用
   `mod tests;` 结束；最终父文件恰好 384 行。
2. **B-002** `src/rules/project/tests.rs` 必须等于基线父文件第 385–629 行逐行去掉四个前导空格
   后，再由当前 toolchain 按 crate edition 执行 `rustfmt --edition 2024` 规范化的结果；最终 child
   恰好 245 行。
3. **B-003** 九个 test names、`ProjectKindCase`、`write_marker`、`assert_matches_targeted`、两套
   cfg-gated `symlink_file` helpers、imports、fixtures 与 assertions 保持不变。
4. **B-004** project-kind matrix、marker order/kind priority、exact-file semantics、snapshot limit/
   missing/invalid/non-UTF-8 behavior、symlink behavior 与 global app cache candidate prefilter contracts
   保持不变。
5. **B-005** implementation diff 只允许修改 `src/rules/project.rs` 并新增
   `src/rules/project/tests.rs`；无其他 source/config/docs/dependency/workflow 变更。
6. **B-006** focused project rule tests、exact relocation proof、full stable/release、精确 Rust 1.95.0、
   VibeGuard、SpecRail、独立 review 与 cross-platform CI gates 通过。

## Edge Cases

- child 中的 `use super::{...}` 必须继续解析到 `rules::project` 私有项，不得扩大 production
  visibility。
- outer `mod tests { ... }` braces 不属于 child；只搬移其 body 并去掉一层缩进。
- `#[cfg(test)]` 必须继续位于父模块声明上，production build 不编译 child。
- 基线 body 不含 `file!`、`line!`、`column!`、`module_path!`、`include!` 或 `#[path]` 等位置敏感
  构造；implementation review 必须再次确认。
- 基线行号只对该 implementation branch 的 `origin/main` 有效；若 main 漂移改变
  `project.rs`，实现必须停止并重新生成 proof，不能套用旧坐标。
- exact proof 必须在 fmt 后运行，把 dedented baseline 经与 `Cargo.toml` 相同的
  `rustfmt --emit stdout --edition 2024` 后再与 child 比较，且 `diff -u` 为空。

## Acceptance Criteria

- B-001 至 B-006 在 tech spec/tasks 中完整映射。
- Spec PR 与 implementation PR 分离，implementation 从 Spec 合并后的最新
  `origin/main` 开始。
- 384/245 line-count 与 exact parent/child equivalence commands 通过。
- 九个 focused tests 和 full gates 通过，无 assertion/test-integrity 弱化。
- current-head CI、签名、reviewThreads、merge state、独立 review 与 SpecRail required gate 全绿后
  才可合并。
