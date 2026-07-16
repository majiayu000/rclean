# GH244 Tasks

## Linked Artifacts

- Issue: `#244`
- Product spec: `specs/GH244/product.md`
- Tech spec: `specs/GH244/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — 等待 Spec PR 人类审查与合并门禁。

## Implementation Tasks

### SP244-T1 — 更新 CLI 概览和模块图

- Owner: `implementation`
- Dependencies: merged GH244 Spec PR
- Covers: B-001, B-002, B-005, B-007
- Change: 删除固定“四个命令”说法；根据当前 `Commands` 和 feature cfg 更新概览；
  把 `plan`/`clean` 记录为 facade 并列出真实子模块职责。
- Done when: 命令面不再错误计数，文档列出的模块/文件全部存在。
- Verify:
  - `rg -n '^enum Commands|^    [A-Z][A-Za-z]+\(' src/cli.rs`
  - `test -f src/plan/revalidate.rs && test -f src/clean/selection.rs && test -f src/clean/roots.rs`
  - `! rg -n 'four CLI commands' docs/architecture.md`

### SP244-T2 — 修正信任边界锚点

- Owner: `implementation`
- Dependencies: SP244-T1
- Covers: B-003, B-006, B-007
- Change: 把旧 facade 内函数定位更新为当前 definition 文件；保证安全承诺文字不
  弱化，不修改任何 Rust 文件。
- Done when: selection、broad-root 和 plan revalidation 锚点指向真实符号，trust
  guarantee 与基线语义一致。
- Verify:
  - `rg -n 'pub fn (select_candidates|check_broad_roots)' src/clean`
  - `rg -n 'pub fn revalidate_selected' src/plan`
  - `git diff --name-only origin/main...HEAD`

### SP244-T3 — 更新当前 sizing 性能模型

- Owner: `implementation`
- Dependencies: SP244-T1
- Covers: B-004, B-005, B-007
- Change: 记录 `SourceSizeIndex` 已落地、candidate `dir_size()` 仍独立遍历，并移除
  过期 milestone 未来时态。
- Done when: 文档不再声称 source lookup 尚未索引，且没有把 source index 错写为
  candidate artifact index。
- Verify:
  - `rg -n 'struct SourceSizeIndex|from_dir_sizes|fn dir_size' src/scan/sizer.rs`
  - `! rg -n 'until that lookup is indexed|Until then' docs/architecture.md`

## Verification And Handoff Tasks

### SP244-T4 — 事实、链接、scope 和格式审查

- Owner: `verification`
- Dependencies: SP244-T1, SP244-T2, SP244-T3
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007
- Done when: 所有变更事实可由当前代码证明，相对链接有效，diff 仅包含
  `docs/architecture.md`，格式 gate 通过，并有当前 head 的 PR gate 证据。
- Verify:
  - `cargo fmt -- --check`
  - `git diff --check`
  - `git diff --name-only origin/main...HEAD`
  - 执行 tech spec 中全部 focused fact checks

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007}`
- Missing invariants: `none`

## Handoff Notes

- implementation 只修改 `docs/architecture.md`，不顺带做性能或代码重构。
- implementation 从本 Spec 合并后的最新 `origin/main` 创建独立分支和 PR。
- 不自行批准、合并或 force push。
