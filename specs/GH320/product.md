# Extract Sizer Tests Into A Dedicated Module - Product Spec

## Linked Issue

- GitHub issue: `#320`
- URL: `https://github.com/majiayu000/rclean/issues/320`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

把 `src/scan/sizer.rs` 中 inline `#[cfg(test)]` block 机械搬移到
`src/scan/sizer/tests.rs`。父模块只保留 `#[cfg(test)] mod tests;`，production sizing
implementation、九个测试、`draft` helper、fixtures 与 assertions 全部不变。拆分并按 crate edition
2024 运行 rustfmt 后，父文件为 477 行，child 为 203 行，显著缩小 production review surface。

## Problem

最新 `origin/main@bbf059c` 的 `src/scan/sizer.rs` 共 681 行：production 结束于第 475 行，第
476–681 行是 inline test module，其中 body 为第 478–680 行、共 203 行。source-size rollup、
serial/parallel equivalence、structured warning ordering 与 saturation tests 和 sizing production code
共享一个 681 行 review surface。

历史 GH109/GH111/GH136/GH240/GH244/GH259/GH284 修改过 sizing behavior、performance 或 tests，
但没有覆盖 test-module extraction。GitHub issues/PRs、`docs/specs/` 与 `specs/` 搜索没有发现同范围
工作，且 `src/scan/sizer/tests.rs` 不存在。仓库已有多处 `module/tests.rs` 组织模式。

## Goals

- 把最大的剩余 inline scan test block 与 production sizing implementation 分离。
- 用 crate-edition-2024 exact proof 保证纯搬移并保持可逆。
- 保留九个测试、`draft` helper 与两个 Unix cfg attributes。
- 不改变 source-byte accounting、parallel walk、warning 或 safety behavior。

## Non-Goals

- 不重写、重命名、增加、删除或合并任何测试/helper/assertion/fixture。
- 不修改 sizing、Rayon、walk、warning aggregation/order、blocked-candidate、saturation 或 safety behavior。
- 不处理 `src/scan/sizer.rs:263` 的既有 `expect()` observation。
- 不改变 visibility、API、CLI、JSON、依赖、feature、workflow、docs 或 trust-model policy。
- 不顺带拆分其他大文件。

## Behavior Invariants

1. **B-001** `src/scan/sizer.rs` 基线第 1–476 行完全不变，第 477 行精确为 `mod tests;`；最终
   parent 恰好 477 行。
2. **B-002** `src/scan/sizer/tests.rs` 等于基线第 478–680 行去掉一层四空格后，经当前 toolchain
   `rustfmt --edition 2024` 规范化的结果；最终 child 恰好 203 行。
3. **B-003** 九个 test names、`draft` helper、imports、fixtures、两个 Unix cfg attributes 与
   assertions 保持不变。
4. **B-004** descendant/sibling source-size rollup、serial/parallel nested-tree equivalence、missing-root
   metadata warning、blocked-candidate no-sizing/no-warning、multi-root partial bytes/stable warnings、Unix
   permission warning sorting、saturating add 与 wide-directory contracts 保持不变。
5. **B-005** implementation diff 只允许修改 `src/scan/sizer.rs` 并新增
   `src/scan/sizer/tests.rs`。
6. **B-006** focused/full stable/release、精确 Rust 1.95.0、VibeGuard、SpecRail、独立 review 与
   current-head cross-platform CI/PR gates 全绿。

## Edge Cases

- child 的 `use super::*` 继续访问 parent 私有项，不扩大 visibility。
- 只搬移 outer wrapper body；`#[cfg(test)]` 保留在 parent declaration。
- Unix-only `PermissionsExt` import 与 permission-warning test cfg 必须原样保留。
- body 不含 `file!`、`line!`、`column!`、`module_path!`、`include!` 或 `#[path]`；实现复核再次扫描。
- 若 implementation base 的 sizer layout 漂移，停止并刷新 proof，不套用旧坐标。
- exact proof 在 fmt 后运行，dedented baseline 必须经与 `Cargo.toml` 相同的 edition 2024 rustfmt。

## Acceptance Criteria

- B-001..B-006 在 tech/tasks 完整映射。
- Spec 与 implementation PR 分离；实现从 Spec 合并后的最新 main 开始。
- 477/203 line-count、exact forward proof 与 reverse rollback proof 通过 stable/MSRV。
- 九个 focused tests 和 full gates 通过，无 test weakening。
- current-head CI、签名、reviewThreads、merge state、独立 review 与 SpecRail gate 全绿后才合并。
