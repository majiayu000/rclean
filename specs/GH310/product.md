# Extract Deletion Tests Into A Dedicated Module - Product Spec

## Linked Issue

- GitHub issue: `#310`
- URL: `https://github.com/majiayu000/rclean/issues/310`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

把 `src/clean/deletion.rs` 中 240 行 inline `#[cfg(test)]` block 机械搬移到
`src/clean/deletion/tests.rs`。父模块只保留 `#[cfg(test)] mod tests;`，生产实现、五个测试、
helpers、assertions、fixtures、timeout 和 mutex 语义全部不变。拆分后生产文件约 308 行，测试
文件约 237 行，两者都回到仓库 200–400 行的典型范围。

## Problem

最新 `origin/main@9e27d30` 的 `src/clean/deletion.rs` 共 546 行：生产实现结束于第 305 行，
第 307–546 行是 inline test module，其中 module body 为第 309–545 行、共 237 行。生产逻辑
本身是聚焦的 306 行，但与 subprocess fixtures、cross-platform fake programs 和测试 helpers
共享一个 546 行 review surface，超过 U-16 典型范围。

历史 #112/#126 已拆分旧的顶层 `clean.rs`，但没有覆盖后来增长的
`src/clean/deletion.rs`。仓库已有 `src/scan/git_cache.rs` +
`src/scan/git_cache/tests.rs` 和 `src/doctor.rs` + `src/doctor/tests.rs` 的相同组织模式。

## Goals

- 让 production deletion module 和 test-only module 各自落入典型行数范围。
- 用 exact textual relocation proof 保证这是一项纯搬移 refactor。
- 保留五个测试及其所有私有 helper 访问、执行顺序约束和跨平台 fixture。
- 缩小后续 production deletion review 的噪声范围。

## Non-Goals

- 不重写、合并、重命名、增加或删除测试/helper/assertion/fixture。
- 不修改 production deletion、native-tool runner、timeout、selection、ActionPlan、graveyard、
  audit、symlink、broad-root 或 protected-path 行为。
- 不改变 visibility、API、CLI、JSON、依赖、feature、workflow、文档或 safety policy。
- 不顺带拆分其他 400–800 行文件。

## Behavior Invariants

1. **B-001** `src/clean/deletion.rs` 原第 1–307 行必须完全不变，并在第 308 行用
   `mod tests;` 结束；最终父文件恰好 308 行。
2. **B-002** `src/clean/deletion/tests.rs` 必须等于基线父文件第 309–545 行逐行去掉四个前导空格
   的结果；最终测试文件恰好 237 行。
3. **B-003** 五个 test names、全部 helpers、imports、常量、30s/50ms/production 60s timeout
   contracts、mutex guard 和七个 error diagnostics 保持不变。
4. **B-004** Unix/Windows fake program bodies、cfg gates、path/env/args/status/stderr predicates 与
   顺序保持不变。
5. **B-005** implementation diff 只允许修改 `src/clean/deletion.rs` 并新增
   `src/clean/deletion/tests.rs`；无其他 source/config/docs/dependency/workflow 变更。
6. **B-006** focused deletion tests、exact relocation proof、full stable/release、精确 Rust 1.95.0、
   VibeGuard、SpecRail、独立 review 与 cross-platform CI gates 通过。

## Edge Cases

- 子模块中的 `use super::*` 必须继续指向 `clean::deletion`，不得为通过编译扩大 production
  visibility。
- outer `mod tests { ... }` braces 不属于新文件内容；只搬移其 body 并去掉一层缩进。
- `#[cfg(test)]` 必须继续位于父模块声明上，production build 不编译新文件。
- 基线行号只对该 implementation branch 的 `origin/main` 有效；若 main 漂移改变
  `deletion.rs`，实现必须停止并重新生成 exact proof，不能继续套用旧行号。
- `cargo fmt` 不能成为隐式重写测试的借口；exact relocation proof 必须在 fmt 后仍为空 diff。

## Acceptance Criteria

- B-001 至 B-006 在 tech spec/tasks 中完整映射。
- Spec PR 与 implementation PR 分离，implementation 从 Spec 合并后的最新
  `origin/main` 开始。
- 父文件/新测试文件的 exact line-count 和 textual equivalence commands 通过。
- 全部测试列表与行为 gates 通过，无 assertion/test-integrity 弱化。
- current-head CI、签名、reviewThreads、merge state、独立 review 与 SpecRail required gate
  全绿后才可合并。
