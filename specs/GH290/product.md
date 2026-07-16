# Feature-gated Audit Variant Warning Cleanup - Product Spec

## Linked Issue

- GitHub issue: `#290`
- URL: `https://github.com/majiayu000/rclean/issues/290`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

让仓库支持的 `no-default`、`tui-only` 与 `graveyard-only` feature 组合在
warnings-as-errors 下保持干净。将仅由 `graveyard` 删除分支构造的 audit enum variants
与同一 feature 边界对齐，并用序列化契约测试证明默认/all-feature 行为不变。

## Problem

`src/clean/deletion.rs` 中 `DeleteAuditMode::Graveyard` 和
`DeleteAuditStatus::Skipped` 的所有构造点都位于 `#[cfg(feature = "graveyard")]` 分支，
但 `src/clean/audit.rs` 对两个 variants 无条件声明。因此在
`origin/main@eebf92a`：

```text
RUSTFLAGS='-D warnings' cargo test --no-default-features --no-run
```

以 exit 101 失败，错误恰好是两个 variants 从未构造。普通 feature-matrix tests 虽通过，
但持续输出 dead-code warnings，无法作为 warnings-clean build 使用。

## Goals

- `graveyard` 关闭时不编译只属于该 feature 的两个 audit variants。
- `graveyard` 开启时两个 variants 继续存在，序列化名称分别保持 `graveyard`、`skipped`。
- base variants 的序列化名称和 `permanent` 推导行为不变。
- warnings-as-errors 下验证纯 no-default、tui-only、graveyard-only 和 all-feature 组合。
- 用确定性 unit test 锁定 audit enum 的 JSON string contract。

## Non-Goals

- 不改变删除选择、Trash、permanent、graveyard move/restore 或 native-tool cleanup。
- 不改变 audit log entry schema、字段、写入/flush、path validation 或 reason。
- 不修改 ActionPlan、安全等级、symlink/broad-root/protected-path gates。
- 不增加 `allow(dead_code)`、`expect(dead_code)` 或全局 lint 抑制。
- 不新增 feature、依赖、CLI flag 或用户配置。

## Behavior Invariants

1. **B-001** 关闭 `graveyard` 时，crate 不包含 `DeleteAuditMode::Graveyard` 和
   `DeleteAuditStatus::Skipped`，且 warnings-as-errors build 无 dead-code error。
2. **B-002** 开启 `graveyard` 时，两个 variants 仍按现有调用点可用，并分别序列化为
   `"graveyard"` 与 `"skipped"`。
3. **B-003** base modes 始终序列化为 `trash`、`permanent`、`go_modcache`、`pip_cache`；
   base statuses 始终为 `success`、`failed`。
4. **B-004** default/all-feature 的现有 graveyard audit log 与 CLI tests 结果不变。
5. **B-005** 修复使用精确 feature cfg，不使用 lint suppression 或 dummy construction。
6. **B-006** diff 只修改 `src/clean/audit.rs` 内 variants 与 focused tests，不改变删除执行代码。
7. **B-007** default、no-default、tui-only、graveyard-only、all-feature、MSRV 和三平台 CI 全部通过。

## Edge Cases

- `graveyard` 与 `tui` 同时关闭：base audit modes/statuses 仍可编译和序列化。
- 只开启 `tui`：不得重新引入 graveyard-only variants 或 warnings。
- 只开启 `graveyard`：`Skipped` 和 `Graveyard` 均可编译并保持现有 snake_case JSON。
- default features：完整 graveyard CLI 测试继续执行。
- serde rename：cfg 不能改变相邻 variants 的序列化名称。

## Acceptance Criteria

- B-001 至 B-007 在 tech spec/tasks 中有完整映射。
- 以下命令均通过且无 warning：
  - `RUSTFLAGS='-D warnings' cargo test --no-default-features --no-run`
  - `RUSTFLAGS='-D warnings' cargo test --no-default-features --features tui --no-run`
  - `RUSTFLAGS='-D warnings' cargo test --no-default-features --features graveyard --no-run`
  - `RUSTFLAGS='-D warnings' cargo test --all-features --no-run`
- focused serialization test 覆盖 base variants，并在 feature enabled 时覆盖两个 gated variants。
- 完整 stable/MSRV/VibeGuard/CI/SpecRail gates 通过。
- Spec PR 与 implementation PR 分离。
