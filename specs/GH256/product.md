# Rule Integration Test Suite Split - Product Spec

## Linked Issue

- GitHub issue: `#256`
- URL: `https://github.com/majiayu000/rclean/issues/256`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

把接近 800 行硬上限的 `tests/rules.rs` 按规则领域拆成同一个 Cargo
integration target 下的子模块，保持现有 36 个测试的行为、函数名和覆盖范围不变，
为后续规则的正例/负例留出清晰且可持续的测试边界。

## Problem

`tests/rules.rs` 当前 769 行，只比仓库 800 行硬上限少 31 行。文件混合了项目构建
产物、全局工具缓存、AI 模型目录、Apple/Xcode 路径和 blocked safety 行为；一次
常规规则新增通常需要正例和负例，足以让该文件越过上限。文件中还出现了一个重复
`#[test]` attribute；它当前没有造成重复注册，但说明跨领域单文件已难以可靠审查。

## Goals

- 保留 Cargo integration target 名称 `rules` 和入口 `tests/rules.rs`。
- 按 project artifacts、tool caches、AI models、platform/safety 领域移动测试。
- 把三个共享 fixture helper 保持为一个私有来源。
- 保留拆分前的 36 个测试函数名、断言、fixture、CLI 参数和预期结果。
- 让入口与每个领域模块低于 400 行，显著远离 800 行硬上限。
- 让机械移动可通过测试清单集合与 moved-code diff 审计。

## Non-Goals

- 不修改 `src/`、生产行为、规则分类、安全级别或删除策略。
- 不新增、删除、重命名、忽略或弱化测试。
- 不重写 fixture、断言、CLI invocation、timeout 或环境变量。
- 不增加依赖、测试框架、宏或 `include!` 拼接。
- 不拆成多个 Cargo integration test binaries。
- 不顺带处理其他测试文件或历史测试重构。

## Behavior Invariants

1. **B-001** 拆分前后 `cargo test --test rules -- --list` 的 bare test-name 集合
   必须相同，包含 36 个唯一测试且每个恰好出现一次。
2. **B-002** 每个测试函数名、测试体、断言、fixture 值、CLI 参数和预期
   rule/safety/category 结果必须保持语义等价；只允许模块前缀和必要 import 变化。
3. **B-003** `make_dir`、`make_non_empty_path` 和 `scan_and_expect_rule` 必须保留
   单一私有来源，由领域模块复用，不得复制或改变行为。
4. **B-004** `tests/rules.rs` 必须继续作为唯一 target `rules` 的入口；使用原 bare
   函数名的 `cargo test --test rules <name>` 过滤仍能运行对应测试。
5. **B-005** 根入口和所有 `tests/rules/*.rs` 文件必须低于 400 行，并按既定领域
   保持内聚；不得用压缩格式或 `include!` 规避文件上限。
6. **B-006** implementation diff 只能修改 `tests/rules.rs` 并新增
   `tests/rules/*.rs`；不得修改生产、依赖、CI 或文档。
7. **B-007** 移动 Xcode clean 测试时只移除冗余的第二个 `#[test]` attribute；该
   test 仍必须在清单中恰好出现一次，测试体不得改变。
8. **B-008** focused rules suite、默认全量测试、三平台 CI、release build 与
   Rust 1.95 build/test 必须保持通过；不得用 skip、ignore 或弱断言解决失败。

## Edge Cases

- 完整 test path 会增加模块前缀，但 bare function-name filter 必须继续命中。
- attribute 必须与被修饰测试原子移动；除 B-007 指定的冗余 attribute 外不得改变。
- shared helper 只对 integration target 内部可见，不能演变为生产 API。
- platform/safety 模块同时含正例、负例和 blocked 断言，移动不能改变安全语义。

## Boundary Checklist

| Boundary | Verdict |
| --- | --- |
| Empty / missing input | N/A：纯测试文件重组，不改变输入契约。 |
| Error and failure paths | Covered by B-002/B-008：负例和 failure 断言保持不变。 |
| Authorization / permission | N/A：不涉及授权。 |
| Concurrency / race / ordering | N/A：现有 suite 无新并发状态。 |
| Retry / repetition / idempotency | Covered by B-001/B-004：清单和过滤结果稳定。 |
| Illegal state transitions | N/A：不引入运行时状态。 |
| Compatibility / migration | Covered by B-004/B-008：target 与过滤接口保留。 |
| Degradation / fallback | Covered by B-006/B-008：不得通过跳过或弱化伪装成功。 |
| Evidence and audit integrity | Covered by B-001/B-002/B-007：集合、唯一性与 moved diff 共同证明。 |
| Cancellation / interruption / partial completion | N/A：没有持久化或长事务。 |

## Acceptance Criteria

- B-001 至 B-008 在 tech spec 和 tasks 中均有确定性验证映射。
- 拆分前后的 36 个 bare test names 集合相同且每个只出现一次。
- `cargo test --test rules` 与代表性领域 filters 通过。
- 所有相关 Rust 文件低于 400 行，implementation scope 仅限指定测试路径。
- Spec PR 只包含 `specs/GH256/`，implementation PR 另行创建。
