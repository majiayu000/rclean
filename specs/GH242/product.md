# CLI Integration Test File Split - Product Spec

## Linked Issue

- GitHub issue: `#242`
- URL: `https://github.com/majiayu000/rclean/issues/242`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

把 1808 行的 `tests/cli.rs` 按 CLI 行为领域拆成同一 integration target 下的
子模块，使每个 Rust 文件重新低于 800 行硬上限，同时保持所有测试行为和
生产代码不变。

## Problem

`tests/cli.rs` 当前包含 61 个测试，覆盖 tmp/home scope、doctor/agent、
scan/clean/plan、free 和生成物输出。文件超过 U-16 硬上限两倍以上，不相关
测试频繁共享同一个冲突面，也让平台/feature cfg 和 helper 的归属不清晰。

## Goals

- 保留 Cargo integration target 名称 `cli`。
- 把测试按已有行为边界移动到 `tests/cli/*.rs` 子模块。
- 保留 61 个测试函数、断言、fixture、cfg 和过滤执行能力。
- 让入口和所有子模块都低于 800 行，并使 helper 留在实际消费者附近。
- 产出可用 moved-code review 的机械 diff。

## Non-Goals

- 不修改任何生产代码或用户可见行为。
- 不修复、重写、增强或弱化测试断言。
- 不改变 fixture 内容、timeout、环境变量或外部命令调用。
- 不新增测试框架、依赖、共享抽象或 `include!` 宏。
- 不把 `cli` 拆成多个 Cargo integration test binaries。

## Behavior Invariants

1. **B-001** 拆分前后的默认 `cargo test --test cli -- --list` 都必须列出 61 个
   tests；不得遗漏、复制、忽略或新增测试。
2. **B-002** 每个原测试函数名、测试体、断言、fixture 值、timeout 及环境变量
   设置必须保持语义等价；允许的变化仅为模块前缀和必要 import 路径。
3. **B-003** 原有 `#[cfg(target_os = ...)]`、`#[cfg(unix)]` 和
   `#[cfg(feature = ...)]` 必须跟随对应测试/helper 移动，不得扩大或缩小平台和
   feature 覆盖。
4. **B-004** `tests/cli.rs` 必须继续作为唯一的 Cargo target `cli` 入口；
   `cargo test --test cli <原函数名>` 必须仍能过滤并运行对应测试。
5. **B-005** `tests/cli.rs` 和每个 `tests/cli/*.rs` 必须低于 800 行；模块按
   tmp scope、home scope、diagnostics/agent、scan/clean/plan、free/generated
   output 等现有领域保持内聚。
6. **B-006** implementation diff 不得修改 `src/`、`Cargo.toml`、测试依赖或 CI
   基础设施，也不得利用拆分夹带行为修复。
7. **B-007** 默认测试、三平台 CI、MSRV 和 #238 修复后的 feature matrix 必须
   保持通过；基线已有失败不得通过弱化或跳过测试解决。

## Edge Cases

- macOS-only helper 必须与其消费者位于同一模块或通过明确私有边界导入。
- TUI-only test 的 feature cfg 必须保留，纯 no-default 构建不得意外编译它。
- 模块前缀会出现在完整 test path 中，但原函数名 substring filter 必须继续命中。
- 根入口不能通过 `include!` 拼接大文件片段规避模块边界。

## Boundary Checklist

| Boundary | Verdict |
| --- | --- |
| Empty / missing input | N/A：纯测试文件重组，不处理输入契约。 |
| Error and failure paths | Covered by B-002/B-007：原失败断言和负例必须原样保留。 |
| Authorization / permission | N/A：不涉及授权。 |
| Concurrency / race / ordering | Covered by B-002：timeout、进程和等待逻辑不得改变。 |
| Retry / repetition / idempotency | Covered by B-001/B-004：重复列举和过滤得到相同测试集合。 |
| Illegal state transitions | N/A：本变更没有运行时状态机。 |
| Compatibility / migration | Covered by B-003/B-004/B-007。 |
| Degradation / fallback | Covered by B-006/B-007：不得用 skip、ignore 或弱断言伪装成功。 |
| Evidence and audit integrity | Covered by B-001/B-002：数量和 moved-code diff 共同证明机械移动。 |
| Cancellation / interruption / partial completion | N/A：没有持久化或长事务。 |

## Acceptance Criteria

- B-001 至 B-007 在 tech spec 和 tasks 中均有验证映射。
- 拆分后的默认 test list 仍为 61，完整 `cargo test --test cli` 通过。
- 代表性 tmp、home、agent、clean、free、TUI filters 均能命中。
- 所有相关 `.rs` 文件低于 800 行。
- Spec PR 只包含 `specs/GH242/`，implementation PR 另行创建。
