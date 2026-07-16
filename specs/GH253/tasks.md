# GH253 Tasks

## Linked Artifacts

- Issue: `#253`
- Product spec: `specs/GH253/product.md`
- Tech spec: `specs/GH253/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — 等待 Spec PR current-head gate 与合并；本文件不代表 implementation
已开始。

## Implementation Tasks

### SP253-T1 — 添加重复 root 与边界回归测试

- Owner: `implementation`
- Dependencies: merged Spec PR for `#253`
- Covers: B-001, B-002, B-003, B-005, B-006, B-007
- Change: 在 `tests/cli/scan_clean.rs` 添加 duplicate literal path、`path/.` alias、
  ancestor/descendant 和 invalid-root fixtures；解析 JSON/ActionPlan 并验证 dry-run。
- Done when: duplicate/alias 测试在未修复 `origin/main` 上稳定失败，边界负例能区分
  exact equality 与 prefix overlap，invalid root 仍显式失败。
- Verify:
  - `cargo test --test cli duplicate_canonical_scan_roots`
  - `cargo test --test cli distinct_canonical_scan_roots_are_preserved`
  - `cargo test --test cli invalid_duplicate_scan_root_is_not_silently_dropped`

### SP253-T2 — 在 scan pipeline 前 canonicalize 并有序去重

- Owner: `implementation`
- Dependencies: SP253-T1
- Covers: B-001, B-002, B-003, B-004, B-006, B-007, B-008
- Change: 在 `src/scan/mod.rs` 使用标准库 membership set + ordered vector 建立 exact
  canonical roots，随后只迭代 unique roots；保留现有错误映射与所有下游逻辑。
- Done when: walker/git/sizer/project pipeline 对每个 exact canonical root 只运行一次，
  first occurrence 顺序稳定，distinct roots 不被合并。
- Verify:
  - `cargo test --test cli duplicate_canonical_scan_roots`
  - `cargo test --test cli distinct_canonical_scan_roots_are_preserved`
  - `cargo test scan::tests`

### SP253-T3 — 验证 report 与 ActionPlan 端到端唯一性

- Owner: `verification`
- Dependencies: SP253-T1, SP253-T2
- Covers: B-004, B-005, B-008
- Change: 核对 human/JSON summary、roots/projects/candidates、plan selected/projects 与
  dry-run 都没有重复项，并运行既有 scan→plan replay 回归。
- Done when: duplicate fixture 的所有结构计数均为 1，dry-run 为
  `Plan: 1 candidates`，既有 single/multi-root 行为保持绿色。
- Verify:
  - `cargo test --test cli duplicate_canonical_scan_roots`
  - `cargo test --test cli scan_write_plan_then_clean_plan_dry_run`
  - `cargo test plan::`

## Verification And Handoff Tasks

### SP253-T4 — 完整 repository 与 PR gate

- Owner: `verification`
- Dependencies: SP253-T1, SP253-T2, SP253-T3
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008
- Done when: focused/full/MSRV checks、spec-vs-implementation 对照、review threads 与
  PR gate 都绑定 implementation PR 当前 head，且 diff 仅包含 planned paths。
- Verify:
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95 cargo build --all-targets --all-features`
  - `rustup run 1.95 cargo test`

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008}`
- Missing invariants: `none`

## Handoff Notes

- implementation 只允许修改 `src/scan/mod.rs` 与 `tests/cli/scan_clean.rs`。
- 不合并 ancestor/descendant roots，不修改 plan writer、schema、classification 或
  clean/delete trust model。
- implementation 必须从 Spec PR 合并后的最新 `origin/main` 创建独立分支。
- 按用户 standing authorization 仅在 current-head gate 全绿后合并；禁止 force push。
