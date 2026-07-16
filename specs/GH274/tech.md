# Watch Polling State Reconciliation - Tech Spec

## Linked Artifacts

- GitHub issue: `#274`
- Product spec: `specs/GH274/product.md`
- Tasks: `specs/GH274/tasks.md`
- Route: `write_spec`

## Codebase Context

| Area | Evidence | Decision |
| --- | --- | --- |
| state store | `WatchState.by_project` 是按 project path 排序的 `BTreeMap` | 保留结构与确定性顺序。 |
| initial scan | `replace_report` 在 run 启动时填充所有有候选项目 | 不改变一次性初始化。 |
| event refresh | lockfile/HEAD 映射到单项目 root | 同一 scope 对账算法兼容精确项目刷新。 |
| polling refresh | 降级后按原始 roots 完整 scan | 把传入 root 作为完整刷新 scope。 |
| current removal | empty report 只 `remove(canonical_key(root))` | 改为移除 scope 内所有 absent keys。 |
| current update | non-empty report 只 insert/update returned projects | 更新前对账缺失 keys。 |
| diff output | `print_diff(project, old, empty)` 已输出每个旧 candidate 的 `removed:` | 复用，不新增输出协议。 |

## Proposed Design

在 `src/watch/mod.rs` 内让 `WatchState::update_project` 执行两个确定性阶段：

1. 从 report 建立当前 project path 集合，并计算 refresh root 的 canonical display key。
2. 遍历 `by_project` 的有序 keys，收集满足以下条件的 stale keys：
   - project path 在 refresh root 的路径组件 scope 内；
   - project path 不在当前 report 集合。
3. 对 stale keys 逐个从 map 删除，并调用
   `print_diff(project, old_candidates, CandidateMap::new())`。
4. 按 report 当前顺序执行现有 insert/update 与 diff。

scope helper 使用 `Path::starts_with` 的组件语义，不用字符串 `starts_with`。refresh root 继续
复用 `canonical_key` 的现有 best-effort canonicalization；本工作不改变文件系统解析或 scan
root 安全策略。report project path 作为 `Path` 比较，但 map key 与公开输出仍保持 `String`。

为了避免边遍历边修改，先把 stale keys clone 到 `Vec<String>`，再执行 removal。`BTreeMap`
保证 removal diff 的顺序稳定。

## Product-to-Test Mapping

| Invariant | Evidence |
| --- | --- |
| B-001 scope reconciliation | state unit test asserts absent in-scope keys removed |
| B-002 partial disappearance | report keeps A while old B is removed |
| B-003 empty broad scope | empty report removes root and descendants |
| B-004 component boundary | `/workspace/a` refresh preserves `/workspace/ab` |
| B-005 outside isolation | unrelated `/other/project` state unchanged |
| B-006 removal diff | pure diff behavior already covered; state test uses old non-empty map and output path remains same caller |
| B-007 current updates | existing diff logic plus test asserting returned project snapshot replaces old value |
| B-008 scope/full gates | exact manifest and fresh stable/release/MSRV/VibeGuard output |

## Planned Changes Manifest

| Path | Change |
| --- | --- |
| `src/watch/mod.rs` | Reconcile refresh scope, add path-boundary helper and focused unit tests. |

No other implementation path is permitted.

## Verification Plan

```sh
cargo test watch::tests -- --nocapture
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
git diff --check
git diff --name-only origin/main...HEAD
```

运行全部已安装 VibeGuard Rust guards。implementation PR current head 需要所有 CI checks 成功、
review threads 为零、merge state 为 CLEAN/MERGEABLE，并由 spec-vs-implementation 检查确认
B-001 至 B-008 无缺失或额外范围。

## Risks And Mitigations

- **字符串前缀误删：** 只用 `Path::starts_with` 组件边界。
- **跨 root 状态误删：** stale filter 同时要求 in-scope 与 absent-from-current-report。
- **空 report 漏删 descendants：** 与 non-empty report 走相同 reconciliation，不保留特殊精确
  key 分支。
- **借用/顺序问题：** 先收集排序后的 cloned keys，再 remove 和打印。
- **输出漂移：** 复用 `print_diff`，不创建新的 diff formatter。
- **scope creep：** 限制为 `src/watch/mod.rs`，拒绝 scan、clean、plan 和 safety 修改。

## Rollback

回滚单个 implementation commit 即恢复原状态更新逻辑；watch 不执行删除，且本改动不写持久化
schema，因此无需数据迁移或恢复步骤。

## Human Gates

- Spec 与 implementation 使用独立 PR。
- 用户已提供本轮 standing merge authorization；仍必须在每次合并前验证 current head、CI、
  review threads、merge state 与 scope，永不 force push。
- 本工作不授权或触发 watch 自动 clean、发布或其他外部副作用。
