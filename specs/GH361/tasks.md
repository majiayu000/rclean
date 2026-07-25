# GH361 Tasks

## Linked Artifacts

- Issue: `#361`
- Product spec: `specs/GH361/product.md`
- Tech spec: `specs/GH361/tech.md`
- Route after maintainer gate: `implement`

## Status

`implemented_pending_review` — maintainer 已选择 A2 并要求开工；实现按真实数据契约
收窄为 interactive selected restore + dry-run + list --older-than，不做
filter-driven bulk restore，不改 restore safety policy 或 schema。聚焦测试、完整
repository gate、MSRV 与 no-default-features gate 已通过，等待 PR current-head 审查。

## Implementation Tasks

### SP361-T1 — CLI 契约与负例

- Owner: `implementation`
- Dependencies: maintainer restore gate
- Covers: B-001, B-002, B-010, B-014
- Change: `RestoreArgs.id` optional、`dry_run`、`--to requires --id`；
  `GraveyardListArgs.older_than`；deferred/unsafe flags 保持不存在。
- Done when: help/parse tests 锁定正负组合，no-default-features 仍不暴露 graveyard CLI。
- Verify:
  - `cargo test default_flow_tests`
  - `cargo build --no-default-features`

### SP361-T2 — 复用 numbered selection 与 confirmation

- Owner: `implementation`
- Dependencies: SP361-T1
- Covers: B-004, B-006, B-015
- Change: crate-visible 复用 `parse_selection`；抽取 `confirm_prompt` 并保持 clean
  文案/行为；restore adapter 支持 `all`/`q`。
- Done when: clean 原测试全绿，restore selection matrix 与 cancellation 在 unit test
  中确定。
- Verify:
  - `cargo test clean::tests`
  - `cargo test graveyard::restore::tests::selection`

### SP361-T3 — 单 ID dry-run 与交互 workflow

- Owner: `implementation`
- Dependencies: SP361-T1, SP361-T2
- Covers: B-001, B-002, B-003, B-005, B-006, B-010, B-011, B-015
- Change: 新增 `src/graveyard/restore.rs` runner；显式 ID dry-run 只 list/find/print；
  无 ID 走 TTY gate、newest-first list、selection、plan、confirm。
- Done when: non-TTY fail closed；dry-run 比较 target parent/payload/manifest 均不变；
  输出使用 `would attempt`。
- Verify:
  - `cargo test graveyard::restore::tests`
  - `cargo test --test graveyard_subcommands restore`

### SP361-T4 — 逐项实际恢复与三分类

- Owner: `implementation`
- Dependencies: SP361-T3
- Covers: B-007, B-008, B-009, B-015
- Change: `restore_selected` 逐项调用既有 `restore_by_id`，分类
  restored/skipped/failed，继续执行并计算 exit。
- Done when: 一项成功 + 一项冲突 fixture 证明 partial completion 可见、冲突 record
  仍在 manifest、exit 1。
- Verify:
  - `cargo test graveyard::restore::tests::batch`
  - `cargo test --test graveyard_subcommands restore`

### SP361-T5 — `list --older-than`

- Owner: `implementation`
- Dependencies: SP361-T1
- Covers: B-012, B-013, B-015
- Change: rendering 前一次性按 age filter；human/JSON 共用 vector；未来 timestamps
  不匹配；空结果 exit 3。
- Done when: fixed-time unit 与 paired CLI fixtures 通过，不读取错误的 `plan_id`
  作为 plan identity。
- Verify:
  - `cargo test graveyard::restore::tests::filter`
  - `cargo test --test graveyard_subcommands graveyard_list`

### SP361-T6 — README / CHANGELOG 与 scope audit

- Owner: `implementation`
- Dependencies: SP361-T3, SP361-T4, SP361-T5
- Covers: B-001, B-010, B-011, B-012, B-014
- Change: 文档新增交互/dry-run/older-than 示例，明确 dry-run 不是 target preflight，
  并明确不支持 plan/bulk/force。
- Done when: docs 不声称 `plan_id` 是共享 plan identity，不使用 `Closes #361`
  以外的扩 scope 表述。
- Verify:
  - `rg -n "restore|older-than|dry-run|plan" README.md CHANGELOG.md specs/GH361`

## Verification And Handoff Tasks

### SP361-T7 — 完整 repository 与 PR gate

- Owner: `verification`
- Dependencies: SP361-T1, SP361-T2, SP361-T3, SP361-T4, SP361-T5, SP361-T6
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009,
  B-010, B-011, B-012, B-013, B-014, B-015
- Done when: focused/full/MSRV/no-default-feature checks、spec-vs-implementation、
  review threads 与 CI 都绑定 PR current head；不执行 merge。
- Verify:
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95 cargo build --all-targets --all-features`
  - `rustup run 1.95 cargo test`
  - `cargo build --no-default-features`

## Invariant Coverage Audit

- Product invariant set:
  `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009,
  B-010, B-011, B-012, B-013, B-014, B-015}`
- Task coverage union:
  `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009,
  B-010, B-011, B-012, B-013, B-014, B-015}`
- Missing invariants: `none`

## Handoff Notes

- Maintainer review gate applies because interactive actual restore writes paths.
- `store.rs`、manifest schema、ActionPlan schema 与 restore safety policy 不在 planned
  paths；发现需要改这些边界时必须停下，不得顺带实现。
- `graveyard list --plan` 暂缓：当前字段是 per-candidate ID，不是 shared plan ID。
- PR must not merge without a separate current-head human authorization.
