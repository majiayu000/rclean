# GH238 Tasks

## Linked Artifacts

- Issue: `#238`
- Product spec: `specs/GH238/product.md`
- Tech spec: `specs/GH238/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — 等待 Spec PR 人类合并门禁；本文件不代表规格已批准。

## Implementation Tasks

### SP238-T1 — 条件化 Clap 互斥关系

- Owner: `implementation`
- Dependencies: merged Spec PR for `#238`
- Covers: B-001, B-002, B-003, B-006
- Change: 仅在 `graveyard` feature 启用时，为 `--permanent` 声明与
  `--graveyard` 的互斥关系；不添加别名或占位参数。
- Done when: 无 `graveyard` 的 `clean` 参数图合法，有 `graveyard` 时两个参数
  仍互斥，scoped diff 不改变删除语义。
- Verify:
  - `cargo run --no-default-features -- clean --help`
  - focused mutual-exclusion parse test

### SP238-T2 — 增加活动 feature 参数图回归测试

- Owner: `implementation`
- Dependencies: SP238-T1
- Covers: B-001, B-002, B-003, B-004
- Change: 增加明确构建并校验完整 Clap command graph 的测试，并断言 feature
  相关参数的存在性与互斥行为。
- Done when: 测试在修复前能暴露无 `graveyard` panic，修复后在四种组合通过。
- Verify:
  - `cargo test --no-default-features cli_graph_is_valid_for_active_features`
  - `cargo test --no-default-features --features tui cli_graph_is_valid_for_active_features`
  - `cargo test --no-default-features --features graveyard cli_graph_is_valid_for_active_features`
  - `cargo test cli_graph_is_valid_for_active_features`

### SP238-T3 — 补齐 CI feature matrix

- Owner: `implementation`
- Dependencies: SP238-T2
- Covers: B-004, B-005, B-006
- Change: 更新 CI，使测试证据覆盖 `none`、`tui`、`graveyard` 和 `default`
  feature 组合；不引入新依赖或外部矩阵工具。
- Done when: implementation PR 的 CI 明确运行至少一个无 `graveyard` 的测试
  组合，且所有组合通过。
- Verify:
  - `cargo test --no-default-features`
  - `cargo test --no-default-features --features tui`
  - `cargo test --no-default-features --features graveyard`
  - `cargo test`

## Verification And Handoff Tasks

### SP238-T4 — 完整验证和规格对照

- Owner: `verification`
- Dependencies: SP238-T1, SP238-T2, SP238-T3
- Covers: B-001, B-002, B-003, B-004, B-005, B-006
- Done when: feature matrix、仓库完整 gate、SpecRail implementation-vs-spec
  对照及 PR gate 都有当前 head SHA 的新鲜证据。
- Verify:
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95 cargo build --all-targets --all-features`
  - `rustup run 1.95 cargo test`

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006}`
- Missing invariants: `none`

## Handoff Notes

- 规格变更和实现必须位于不同 PR。
- implementation 分支必须从 Spec PR 合并后的最新 `origin/main` 创建。
- 不自行批准、合并或 force push。
