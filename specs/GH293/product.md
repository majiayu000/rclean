# Watch ActionPlan Collision Prevention - Product Spec

## Linked Issue

- GitHub issue: `#293`
- URL: `https://github.com/majiayu000/rclean/issues/293`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

保证 `rclean watch --write-plan <base>` 在同一秒内发生多次刷新时，每次成功刷新都写入
不同的 ActionPlan 文件。保留现有可读时间戳前缀；当秒级文件名已存在时使用确定性数字
后缀，而不是静默覆盖较早的 plan。

## Problem

`src/watch/mod.rs::timestamped_path` 只使用 `%Y%m%dT%H%M%SZ` 秒级时间戳。
`write_timestamped_plan` 随后调用会原子替换既有目标的 ActionPlan writer，因此同一秒内
的多个 refresh 会反复写同一路径。最新 `origin/main@0b3d648` 的真实复现中，49 条
`wrote action plan` 消息只产生 2 个文件，其中 48 次写入同一个路径。

这与已发布 `docs/specs/v0.1.x-roadmap.md` §4.8 的“每次刷新写一份新 plan（带时间戳
后缀）”不一致，并静默丢失 watch 审计历史。

## Goals

- 同一 watch 进程的每次成功刷新选择一个尚不存在的 plan 路径。
- 保留当前未碰撞文件名、UTC 秒级时间戳和扩展名行为。
- 碰撞时按 `-2`、`-3`……选择确定性后缀。
- 检查目标是否存在失败时显式报错，且不得打印成功消息。
- 用确定性测试覆盖同一时间戳的连续碰撞，无需依赖墙钟或 sleep。

## Non-Goals

- 不改变 ActionPlan schema、内容、原子写入实现、读取或 replay。
- 不改变 notify 事件过滤、lockfile 映射、20 分钟 polling 降级或 scan 行为。
- 不改变 clean/delete、graveyard、symlink、broad-root 或 protected-path policy。
- 不承诺多个独立 `rclean watch` 进程并发写同一 base path 的跨进程互斥；本改动处理
  单进程串行 refresh 与启动时已存在文件。
- 不增加随机依赖、UUID/ULID schema、CLI flag 或配置。

## Behavior Invariants

1. **B-001** 首个未占用候选继续使用 `<stem>-<UTC-second>.<ext>`，没有额外后缀。
2. **B-002** 首个候选已存在时选择 `<stem>-<UTC-second>-2.<ext>`；继续碰撞时单调选择
   `-3`、`-4`，且任何既有文件内容不变。
3. **B-003** 同一进程每次成功 plan write 对应唯一目标路径和一个成功消息。
4. **B-004** `.json` 及无扩展名 base 都保持当前 stem/extension 规则；非 UTF-8 stem
   继续使用 `rclean-watch` fallback。
5. **B-005** `try_exists`/metadata 错误显式返回结构化 plan I/O error，不得把 unknown
   当作 unused，也不得继续写 plan 或打印成功。
6. **B-006** plan schema/content、atomic writer、watch refresh/diff/scan 语义和信任模型
   完全不变。
7. **B-007** diff 只修改 `src/watch/mod.rs`，且 focused、stable、MSRV、VibeGuard 与
   三平台 CI 全部通过。

## Edge Cases

- 同一秒只有一次刷新：文件名与当前版本相同。
- 同一秒连续三次刷新：生成无后缀、`-2`、`-3` 三个文件。
- 目标目录中已留有前一次运行的同秒文件：从首个未占用后缀继续，不覆盖旧文件。
- base 为 `auto.json`：生成 `auto-<stamp>.json`、`auto-<stamp>-2.json`。
- base 无 extension：生成 `auto-<stamp>`、`auto-<stamp>-2`。
- 存在性检查被权限或 I/O 错误阻断：命令显式失败并保留已有文件。
- 后缀计数溢出：显式失败；不得回绕为已有路径。

## Acceptance Criteria

- B-001 至 B-007 在 tech spec/tasks 中有完整映射。
- 固定 timestamp 的测试证明首选、`-2`、`-3` 路径和旧文件内容不变。
- extensionless 与 non-UTF-8 fallback 行为有 focused coverage。
- 存在性检查错误与计数溢出路径不静默降级。
- `cargo test watch::tests` 及完整 stable/MSRV/VibeGuard/CI/SpecRail gates 通过。
- Spec PR 与 implementation PR 分离。
