# Architecture Documentation Refresh - Product Spec

## Linked Issue

- GitHub issue: `#244`
- URL: `https://github.com/majiayu000/rclean/issues/244`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

更新 `docs/architecture.md` 的当前实现快照，使 CLI 概览、模块图、信任边界锚点和
扫描性能模型与当前 `main` 一致，同时把历史路线图与当前架构事实明确分开。

## Problem

架构文档仍声称项目只有四个 CLI commands，把已经拆分的 `plan` 和 `clean` 描述为
单体文件，并引用已迁移的信任边界符号。性能章节也仍把 source-byte 索引写成未来
工作，尽管 `SourceSizeIndex` 已经落地。新贡献者据此定位代码或评估性能改动时会
得到错误答案。

## Goals

- 让 CLI 概览准确表达当前命令面，避免会迅速过期的固定数量。
- 让模块图表达 `plan.rs`/`clean.rs` facade 与对应子模块的职责边界。
- 让每个信任边界锚点指向当前存在的文件和符号。
- 让性能章节准确描述 `SourceSizeIndex`、candidate 独立 sizing 和当前热点。
- 区分历史路线图与当前实现快照，删除已经实现事项的未来时态。

## Non-Goals

- 不修改 Rust 生产代码、测试、CI、依赖或 feature 配置。
- 不改变 CLI、JSON/ActionPlan schema、扫描、选择或删除行为。
- 不放宽 broad-root、symlink、revalidation、blocked 或 dirty-git 保证。
- 不重写完整架构文档，也不新增未经代码证据支持的未来设计。
- 不在本工作中修复文档所描述的性能热点。

## Behavior Invariants

1. **B-001** 开篇和 CLI 概览不得再声称固定的“四个命令”；描述必须覆盖当前顶层
   命令面，且不把 feature-gated 命令写成所有构建无条件存在。
2. **B-002** 模块图必须把 `src/plan.rs`、`src/clean.rs` 表达为 facade，并列出
   `src/plan/`、`src/clean/` 中承担 schema、I/O、revalidation、selection、root
   guard、confirmation/output 和 deletion 的真实边界。
3. **B-003** trust-boundary 表中所有文件和符号锚点必须在当前代码中存在；已有
   安全保证的含义不得改变或弱化。
4. **B-004** Performance shape 必须说明 `SourceSizeIndex` 已从 `DirSizes` 建立一次
   索引供 project/source 查询，并保留 candidate directories 被独立 `dir_size()`
   遍历这一当前热点；不得保留“尚待索引”的描述。
5. **B-005** 当前架构事实与历史 `v0.1.x` 路线图必须明确分离；已经落地的
   milestone 不得继续以 “until then” 等未来时态出现。
6. **B-006** implementation 只允许修改 `docs/architecture.md`；运行时、测试、
   schema、依赖、CI 和安全策略保持不变。
7. **B-007** 新增或更新的每个路径、模块名、函数名和相对链接都必须通过当前
   `origin/main` 的文件/符号搜索或链接检查验证，不得凭记忆补写。

## Edge Cases

- `tui`、`watch`、`graveyard` 等 feature-gated 命令需要明确条件性，避免把完整
  feature build 与 no-default build 混为一谈。
- facade 仍是有效公开模块入口；文档不能因强调子模块而暗示调用方应绕过 facade。
- `SourceSizeIndex` 只覆盖 source/project bytes；candidate artifact bytes 仍单独
  遍历，不能把两者错误合并成“一次 walk 完成所有 sizing”。
- 历史文档链接可以保留，但其状态必须标成历史规划背景而非当前未完成承诺。

## Boundary Checklist

| Boundary | Verdict |
| --- | --- |
| Empty / missing input | N/A：文档准确性变更，不处理运行时输入。 |
| Error and failure paths | Covered by B-003：错误/拒绝路径的信任保证不得被文档弱化。 |
| Authorization / permission | N/A：不改变授权。 |
| Concurrency / race / ordering | Covered by B-004：只描述现有两阶段 sizing，不改执行顺序。 |
| Retry / repetition / idempotency | N/A：无运行时行为。 |
| Illegal state transitions | Covered by B-003：ActionPlan revalidation 保证保持原意。 |
| Compatibility / migration | Covered by B-001/B-002/B-005/B-006。 |
| Degradation / fallback | Covered by B-006：没有运行时 fallback。 |
| Evidence and audit integrity | Covered by B-007：所有事实必须有当前代码证据。 |
| Cancellation / interruption / partial completion | N/A：无持久化或长事务。 |

## Acceptance Criteria

- B-001 至 B-007 在 tech spec 和 tasks 中均有验证映射。
- `docs/architecture.md` 不再包含已确认的四类漂移。
- 受影响路径、符号和链接均经当前 `origin/main` 验证。
- Spec PR 只包含 `specs/GH244/`；implementation PR 另行创建且只改架构文档。
