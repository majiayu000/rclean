# GH246 Tasks

## Linked Artifacts

- Issue: `#246`
- Product spec: `specs/GH246/product.md`
- Tech spec: `specs/GH246/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — 等待 Spec PR 人类审查与合并门禁。

## Implementation Tasks

### SP246-T1 — 补齐 home scope 并修正 Go safety

- Owner: `implementation`
- Dependencies: merged GH246 Spec PR
- Covers: B-001, B-002, B-003, B-004, B-005, B-007
- Change: 为 Go module root 和 7 个 macOS user-level caches 添加 scope/path/safety/
  restore 表项，把现有 Go download row 从 safe 改为 caution/native cleanup，并修正
  surrounding full-list 说明。
- Done when: 8 个 home ids 具有 canonical rows，两个 Go module-cache rows 都标为
  caution/native cleanup，macOS rows 与 classifier safety 一致。
- Verify:
  - 执行 tech spec Expected-ID Check 的 home 子集。
  - `rg -n 'home_toolchain_paths' src/cli.rs`
  - `rg -n 'go\.module_(download_)?cache.*caution' README.md`
  - `rg -n 'Safety::(Safe|Caution)' src/rules/go.rs src/rules/app_caches.rs src/rules/macos_system.rs`

### SP246-T2 — 记录 tmp 与 explicit-path 规则边界

- Owner: `implementation`
- Dependencies: SP246-T1
- Covers: B-001, B-002, B-003, B-004, B-007
- Change: 记录两个 `--tmp` rules 和 code-sign clone explicit scan rule；保留 marker、
  include-caution 和 closed-process 提示。
- Done when: README 不声称 code-sign clone 的 `X` path 被默认 `--tmp` 扫描，整个
  worktree candidate 明确为 caution。
- Verify:
  - `rg -n 'tmp_workspace_paths|/private/tmp|/tmp' src/cli.rs`
  - `rg -n 'macos\.chrome_code_sign_clone|macos\.remem_dry_run_tmp|agent\.tmp_worktree' README.md src/rules tests/cli.rs`

### SP246-T3 — 记录 system report-only 边界

- Owner: `implementation`
- Dependencies: SP246-T1
- Covers: B-001, B-002, B-003, B-004, B-007
- Change: 添加 `apple.idleassetsd` system row，明确 exact anchor、report-only、管理员
  权限和 never selected。
- Done when: README 不暗示 rclean 会执行 sudo 或 clean 该 path。
- Verify:
  - `rg -n 'apple\.idleassetsd|ReportOnly|IDLEASSETSD_PATH' src/rules/macos_system.rs`
  - `rg -n 'apple\.idleassetsd|report-only|--system' README.md`

## Verification And Handoff Tasks

### SP246-T4 — 运行确定性覆盖、scope 和 docs-only gate

- Owner: `verification`
- Dependencies: SP246-T1, SP246-T2, SP246-T3
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007
- Done when: 12 个 ids 均有 canonical entries，四种发现 scope 和 global table 全量
  safety 经代码证据人工复核且无剩余冲突，diff 仅为 README，格式与 PR gate 具有
  当前 head 证据。
- Verify:
  - 执行 tech spec 中完整 Expected-ID Check、全表 safety comparison 和 Focused fact checks。
  - `cargo fmt -- --check`
  - `git diff --check`
  - `test "$(git diff --name-only origin/main...HEAD)" = "README.md"`

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007}`
- Missing invariants: `none`

## Handoff Notes

- implementation 只改 README，不顺带修改规则、root expansion 或 tests。
- implementation 从本 Spec 合并后的最新 `origin/main` 创建独立分支和 PR。
- 不自行批准、合并或 force push。
