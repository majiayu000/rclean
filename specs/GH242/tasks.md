# GH242 Tasks

## Linked Artifacts

- Issue: `#242`
- Product spec: `specs/GH242/product.md`
- Tech spec: `specs/GH242/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — 等待 Spec PR 与 #238 implementation 人类合并门禁。

## Implementation Tasks

### SP242-T1 — 建立 cli integration 子模块骨架

- Owner: `implementation`
- Dependencies: merged GH242 Spec PR; #238 implementation merged to `main`
- Covers: B-003, B-004, B-005, B-006
- Change: 保留 `tests/cli.rs` Cargo target，声明五个普通私有模块，并为每个模块
  添加最小必要 imports；不使用 `include!` 或新依赖。
- Done when: root/模块结构编译，scope 仅限 `tests/cli.rs` 与 `tests/cli/*.rs`。
- Verify:
  - `cargo test --test cli --no-run`
  - `git diff --name-only origin/main...HEAD`

### SP242-T2 — 按行为领域机械移动 61 个 tests

- Owner: `implementation`
- Dependencies: SP242-T1
- Covers: B-001, B-002, B-003, B-004, B-005
- Change: 以 attribute + helper/test item 为原子单位移动到 tmp、home、diagnostics、
  scan/clean、free/output 模块；不编辑断言或 fixture 语义。
- Done when: list 仍为 61 tests，原函数名集合完整，所有文件低于 800 行。
- Verify:
  - `cargo test --test cli -- --list`
  - `find tests/cli -name "*.rs" -print0 | xargs -0 wc -l`
  - `wc -l tests/cli.rs`
  - `git diff --color-moved=dimmed-zebra origin/main...HEAD`

### SP242-T3 — 运行代表性 filters 和完整 cli suite

- Owner: `verification`
- Dependencies: SP242-T2
- Covers: B-001, B-002, B-003, B-004, B-007
- Done when: tmp、home、agent、clean、free 和 feature-gated 测试仍被发现，完整
  `cli` target 通过。
- Verify:
  - `cargo test --test cli`
  - `cargo test --test cli tmp_flag_scans_rust_targets_under_temp_worktree`
  - `cargo test --test cli home_flag_reports_user_tool_safe_caches`
  - `cargo test --test cli agent_doctor_json_runs_for_codex`
  - `cargo test --test cli clean_dry_run_does_not_delete`
  - `cargo test --test cli free_target_met_writes_plan_and_exits_zero`

## Verification And Handoff Tasks

### SP242-T4 — 完整 gate 和机械 diff 审查

- Owner: `verification`
- Dependencies: SP242-T1, SP242-T2, SP242-T3
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007
- Done when: full/default/feature/MSRV gates 通过，非移动 hunks 仅限 imports 和 module
  wiring，规格对照与 PR gate 具有当前 head 的证据。
- Verify:
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `cargo test --no-default-features`
  - `cargo test --no-default-features --features tui`
  - `cargo test --no-default-features --features graveyard`
  - `rustup run 1.95 cargo build --all-targets --all-features`
  - `rustup run 1.95 cargo test`

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007}`
- Missing invariants: `none`

## Handoff Notes

- 不修改生产代码、CI、依赖或测试断言。
- implementation 从 #238 修复与本 Spec 合并后的最新 `origin/main` 创建。
- 不自行批准、合并或 force push。
