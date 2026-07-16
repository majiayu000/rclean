# Watch Polling State Reconciliation - Product Spec

## Linked Issue

- GitHub issue: `#274`
- URL: `https://github.com/majiayu000/rclean/issues/274`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `medium`

## Summary

让 `rclean watch` 的每次增量或轮询刷新都把指定 root 当作完整刷新 scope：scope 内已从
最新扫描消失的项目必须从内存状态移除并输出现有 `removed:` diff，scope 外状态保持不变。

## Problem

`WatchState::update_project` 目前只处理两种情况：report 完全为空时删除与 root 精确相等的
单个 key；report 非空时只插入/更新仍出现的项目。轮询一个包含 A、B 多个项目的 root 时，
若 B 的候选消失而 A 仍有候选，B 的旧快照会永久保留。若整个 broad root 的 report 为空，
其子项目 key 也不会被精确 root 删除。

结果是长时间运行的 watch 输出和内存状态可能与当前 scan 不一致，且用户看不到 B 的
`removed:` diff。

## Goals

- 每次 refresh 对指定 root 的完整 scope 做状态对账。
- 非空 report 中缺失的旧项目从 scope 状态移除。
- 空 report 移除 scope 内所有旧项目，而不只删除精确 root key。
- 每个被移除项目继续使用现有 candidate diff 输出。
- 单项目 lockfile/`.git/HEAD` 事件刷新保持正确。
- 使用路径组件边界判断 scope，避免字符串前缀碰撞。
- 保持 scope 外项目状态不变。

## Non-Goals

- 不改变 notify 监听文件、20 分钟降级阈值或 `--every` 语义。
- 不新增后台清理、自动选择或 graveyard 写入。
- 不改变 scan 发现、候选大小、安全分级、risk score 或 git dirty 行为。
- 不改变 ActionPlan schema、写入、重放或时间戳命名。
- 不重构 watch 输出格式或增加新的 CLI flag。
- 不改变 symlink、canonicalization、root boundary 或删除安全策略。

## Behavior Invariants

1. **B-001** refresh root 定义本次完整状态 scope；scope 内旧项目若不在新 report 中，必须
   从 `WatchState.by_project` 移除。
2. **B-002** report 非空时仍要移除缺失的 sibling 项目，不能只更新返回的项目。
3. **B-003** report 为空时要移除 refresh root 自身及其所有已记录 descendant 项目。
4. **B-004** scope 判断使用路径组件边界；刷新 `/workspace/a` 不得影响
   `/workspace/ab`。
5. **B-005** scope 外项目与候选快照保持不变。
6. **B-006** 每个被移除项目调用既有 diff 语义，旧候选各输出一次 `removed:`；无候选的旧
   项目允许只从状态消失，不新增噪音格式。
7. **B-007** 当前 report 中的项目继续插入/更新，并保留 `added`、`changed`、`refreshed`
   行为。
8. **B-008** 实现只修改 watch 状态协调和同模块回归测试；stable、release、MSRV 与
   VibeGuard gate 全部通过。

## Acceptance Criteria

- B-001 至 B-008 在 tech spec 与 tasks 中完整映射。
- 回归测试先证明当前 main 在“非空 partial disappearance”场景失败。
- 测试覆盖空 broad scope、路径前缀隔离、scope 外状态保留和当前项目更新。
- implementation diff 只包含 `src/watch/mod.rs`；Spec PR 与 implementation PR 分离。
- 不触及任何 trust-model 删除或 ActionPlan gate。
