# Architecture Documentation Refresh - Tech Spec

## Linked Artifacts

- GitHub issue: `#244`
- Product spec: `specs/GH244/product.md`
- Tasks: `specs/GH244/tasks.md`
- Route: `write_spec`

## Codebase Context

| Area | Current evidence | Decision |
| --- | --- | --- |
| `docs/architecture.md:4` | 写成 “four CLI commands”。 | 改为不依赖固定数量的当前命令面描述，并注明 feature-gated commands。 |
| `src/cli.rs:21` | `Commands` 包含 agent、docker、scan、clean、free、TUI、watch、stamp、explain、completions、doctor 和 graveyard commands。 | 用当前 enum 验证文档，不复制易漂移的精确计数。 |
| `docs/architecture.md:50` | `plan.rs`、`clean.rs` 被描述为全部实现。 | 将二者标为 facade，并列出 `plan/`、`clean/` 关键子模块。 |
| `src/plan.rs`, `src/clean.rs` | 当前是 re-export facade。 | 保留 facade 作为入口，不建议绕过公开边界。 |
| `docs/architecture.md:152` | trust-boundary 表引用旧的 facade 内函数路径。 | 更新到 `clean/selection.rs`、`clean/roots.rs`、`plan/revalidate.rs` 等真实定义位置。 |
| `docs/architecture.md:188` | 写成 source-byte lookup “until ... indexed”。 | 描述 `SourceSizeIndex::from_dir_sizes()` 已建立排序前缀索引。 |
| `src/scan/sizer.rs:29` | `SourceSizeIndex` 已实现 project/source subtree 查询。 | 明确 source lookup 与 candidate `dir_size()` 是不同路径。 |
| `docs/specs/v0.1.x-roadmap.md` | 历史 minor-series roadmap。 | 作为历史背景链接保留，不再写成当前等待落地事项。 |

## Proposed Documentation Changes

1. 改写开篇命令面说明，不再使用固定的“四个命令”。
2. 更新 module map：
   - `plan.rs` 和 `clean.rs` 标为 facade；
   - 增加 `plan/` 的 schema、I/O、selection/revalidation、ID 边界；
   - 增加 `clean/` 的 selection、broad-root guard、confirmation/output、deletion
     和 result types 边界。
3. 更新 trust-boundary 表的符号位置，保持保证文字语义不变。
4. 改写 Performance shape：
   - walk phase 收集 `DirSizes`；
   - `SourceSizeIndex` 一次构建后服务多个 project/source range 查询；
   - candidate artifacts 因 pruning 仍由独立 `dir_size()` 遍历；
   - 性能改动继续要求 benchmark/wall-clock evidence，但不使用过期 milestone
     未来时态。
5. 调整相关文档说明，把 `v0.1.x` roadmap 标记为历史规划背景。

## Product-to-Verification Mapping

| Behavior invariant | Documentation area | Verification |
| --- | --- | --- |
| B-001 当前 CLI 面 | introduction, command sections | 对照 `src/cli.rs::Commands`；搜索并移除 `four CLI commands` |
| B-002 facade/submodule 边界 | module map | `test -f` 检查列出的文件；对照 `src/plan.rs`、`src/clean.rs` re-exports |
| B-003 trust boundaries | trust-boundary table | `rg` 验证 selection、root guard、revalidation 和 safety symbols |
| B-004 当前 sizing 模型 | Performance shape | 对照 `SourceSizeIndex`、`summarize`、`dir_size` 和 walker pruning |
| B-005 历史/当前分离 | introduction, performance, related docs | 搜索过期 `Until then`/当前 milestone 表述并人工审查语境 |
| B-006 docs-only scope | entire implementation diff | `git diff --name-only origin/main...HEAD` 仅为 `docs/architecture.md` |
| B-007 事实与链接完整 | all changed lines | 路径/符号 `rg` + Markdown relative-link checker |

## Risks And Mitigations

- **风险：** 为避免固定数量而把命令面写得过于模糊。**缓解：** 保留按主要 pipeline
  分组的命令说明，并明确 feature-gated commands。
- **风险：** 更新符号路径时无意改变安全承诺。**缓解：** 只改 `Where` 定位，逐行
  对比 `What it guarantees` 文本。
- **风险：** 把 `SourceSizeIndex` 描述成 candidate size 索引。**缓解：** 明确区分
  project/source range sum 与 candidate `dir_size()`。
- **风险：** 文档路径未来再次漂移。**缓解：** 优先记录稳定模块职责，同时用当前
  符号搜索验证精确锚点。

## Verification Plan

Focused fact checks:

```sh
rg -n '^enum Commands|Agent\(|Docker\(|Scan\(|Clean\(|Free\(|Tui\(|Watch\(|Stamp\(|Explain\(|Completions\(|Doctor\(|Restore\(|Graveyard\(' src/cli.rs
rg -n 'pub use .*select_candidates|pub use .*check_broad_roots|pub use .*revalidate_selected' src/clean.rs src/plan.rs
rg -n 'pub fn (select_candidates|check_broad_roots|revalidate_selected)' src/clean src/plan
rg -n 'struct SourceSizeIndex|from_dir_sizes|fn dir_size|SourceSizeIndex' src/scan/sizer.rs
! rg -n 'four CLI commands|until that lookup is indexed|Until then' docs/architecture.md
```

File and scope checks:

```sh
test -f src/clean/selection.rs
test -f src/clean/roots.rs
test -f src/plan/revalidate.rs
test -f src/scan/sizer.rs
git diff --check
git diff --name-only origin/main...HEAD
```

Repository docs gate:

```sh
cargo fmt -- --check
```

Rust build/test are not required for the implementation because B-006 restricts the diff to
Markdown and no executable input changes. Existing CI may still run the repository gate.

## Rollback

本变更只更新一份 Markdown 快照。若事实或链接验证失败，回滚 implementation commit
即可；没有运行时、schema、数据或安全策略迁移。

## Human Gates

- Spec PR 与 implementation PR 分离。
- Spec PR 人工合并后，implementation 从当时最新 `origin/main` 创建。
- 不自行批准、合并或 force push。
