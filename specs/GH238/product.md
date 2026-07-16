# `--no-default-features` Feature Matrix - Product Spec

## Linked Issue

- GitHub issue: `#238`
- URL: `https://github.com/majiayu000/rclean/issues/238`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

恢复仓库已经承诺支持的 `--no-default-features` 运行方式，确保关闭
`graveyard` feature 后，`clean` 子命令及其帮助仍能正常解析，而不是在
Clap 参数图校验阶段 panic。

## Problem

`clean --permanent` 当前无条件引用 `graveyard` 参数作为互斥目标，但该参数
会在 `graveyard` feature 关闭时从 CLI 中移除。结果是纯
`--no-default-features` 和 `tui`-only 构建虽然能够编译，任何进入 `clean`
参数图的调用都会以 exit `101` panic。现有 CI 只检查启用了 `graveyard` 的
最小构建，因此没有覆盖真正缺失该参数的组合。

## Goals

- 让所有受支持的 feature 组合都能构建合法的 `clean` 参数图。
- 保留启用 `graveyard` 时 `--permanent` 与 `--graveyard` 的互斥行为。
- 用自动化测试持续覆盖没有 `graveyard` 的组合，避免默认 feature 掩盖回归。
- 保持现有扫描、选择、删除、ActionPlan 和安全策略不变。

## Non-Goals

- 不新增、删除或重命名 CLI 参数与 feature。
- 不改变默认删除模式、确认流程或 `--permanent` 的语义。
- 不改变 `graveyard` 的存储、恢复、GC 或 ActionPlan 行为。
- 不借此整理无关的 feature 警告、依赖或代码风格。

## Behavior Invariants

1. **B-001** 当 `graveyard` feature 关闭时，`clean --help`、显式
   `clean` 调用和无子命令默认流程必须构建合法的 Clap 参数图，不得 panic。
2. **B-002** 当 `graveyard` feature 关闭时，`--permanent` 必须继续可用，
   `--graveyard` 必须保持不存在；不得因为移除互斥目标而移除或禁用
   `--permanent`。
3. **B-003** 当 `graveyard` feature 启用时，`--permanent` 与
   `--graveyard` 必须继续互斥，同时单独使用任一参数仍可正常解析。
4. **B-004** 受支持的 feature 组合 `{none, tui, graveyard, default}` 必须都能
   完成 CLI 参数图校验和测试；其中 `default` 等价于当前默认 feature 集合。
5. **B-005** CI 必须至少执行一个真正关闭 `graveyard` 的测试组合；只运行
   `--no-default-features --features graveyard` 不足以作为无默认 feature 支持证据。
6. **B-006** 修复不得改变参数名称、帮助文本契约、退出码、JSON schema、
   ActionPlan schema、候选选择或删除安全分类。

## Edge Cases

- 仅请求顶层 `--help` 可能不会展开 `clean` 参数图，不能作为 B-001 的唯一证据。
- `tui`-only 会关闭 `graveyard`，必须与纯 `none` 组合得到相同的合法参数关系。
- `graveyard`-only 没有 TUI，但仍必须保留 `--permanent` / `--graveyard` 互斥。
- 默认 feature 同时启用 `graveyard` 和 `tui`，现有行为必须保持不变。

## Boundary Checklist

| Boundary | Verdict |
| --- | --- |
| Empty / missing input | Covered by B-001: 无子命令默认流程也必须能够解析。 |
| Error and failure paths | Covered by B-001 and B-003: 参数图不得 panic，互斥输入仍由 Clap 拒绝。 |
| Authorization / permission | N/A：本变更只构建本地 CLI 参数图，不涉及授权。 |
| Concurrency / race / ordering | N/A：Clap 参数图构建是单进程确定性操作。 |
| Retry / repetition / idempotency | N/A：重复解析同一参数不改变任何持久状态。 |
| Illegal state transitions | Covered by B-003: 两个删除模式参数同时出现仍是非法组合。 |
| Compatibility / migration | Covered by B-002、B-004、B-006。 |
| Degradation / fallback | Covered by B-002 and B-005: 不得通过移除 `--permanent` 或仅验证含 `graveyard` 的组合伪装成功。 |
| Evidence and audit integrity | Covered by B-004 and B-005: 测试证据必须来自实际 feature 组合。 |
| Cancellation / interruption / partial completion | N/A：参数解析和离线测试没有可恢复的长事务。 |

## Acceptance Criteria

- B-001 至 B-006 均由 `tech.md` 中的确定性检查覆盖。
- 四种 feature 组合的测试命令均通过。
- 默认仓库验证保持通过。
- Spec PR 只包含 `specs/GH238/`，implementation PR 另行创建。
