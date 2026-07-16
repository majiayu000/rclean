# CLI Integration Test File Split - Tech Spec

## Linked Artifacts

- GitHub issue: `#242`
- Product spec: `specs/GH242/product.md`
- Tasks: `specs/GH242/tasks.md`
- Route: `write_spec`

## Codebase Context

| Area | Current evidence | Decision |
| --- | --- | --- |
| `tests/cli.rs:1` | 单一 integration test crate 入口和全局 imports。 | 保留文件作为 `cli` target，只声明私有子模块。 |
| `tests/cli.rs:19` | tmp/home scope 参数和扫描测试开始。 | tmp scope 测试移动到独立模块。 |
| `tests/cli.rs:431` | home/global cache expansion 测试开始。 | home scope 测试和其 macOS helper 移到独立模块。 |
| `tests/cli.rs:919` | doctor/help/agent/watch/TUI 测试开始。 | diagnostics/agent 模块。 |
| `tests/cli.rs:1107` | scan/clean/plan/rules/explain 测试开始。 | scan/clean/plan 模块。 |
| `tests/cli.rs:1625` | free/progress/completions/man 测试开始。 | free/generated output 模块。 |
| `tests/cli.rs:1808` | 文件结尾；当前总计 1808 行和 61 tests。 | 所有目标文件必须小于 800 行。 |

## Proposed Module Layout

```text
tests/
├── cli.rs                 # 仅 mod declarations；Cargo target 仍名为 cli
└── cli/
    ├── tmp_scope.rs       # tmp、broad-root、open-file gate
    ├── home_scope.rs      # home expansion、global/app cache reporting
    ├── diagnostics.rs     # doctor、help、agent、watch、TUI fallback
    ├── scan_clean.rs      # scan、clean、plan、rules、explain、summary
    └── free_output.rs     # free、progress JSON、completions、man
```

模块使用普通 `mod`，不使用 `include!`。每个模块只导入自身需要的
`assert_cmd`、`predicates`、`serde_json`、platform std types 和 `TempDir`。
helper 与唯一消费者一起移动；没有三次以上重复前不提取共享 abstraction。

## Mechanical Move Rules

1. 先建立空模块声明和所需 imports，再以完整测试函数/helper 为单位移动。
2. 不编辑函数体；只有 Rust module scope 所需的 import 和可见性调整允许变化。
3. 保留所有 attribute 与被修饰 item 相邻移动。
4. 不重命名测试函数。完整路径新增模块前缀是预期的内部变化，bare function
   substring filter 仍兼容。
5. 用 `git diff --color-moved=dimmed-zebra` 人工确认主体是移动，而非重写。

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 仍为 61 tests | all new modules and root | `cargo test --test cli -- --list` then `rg -c ": test$"` equals `61` |
| B-002 函数体/断言/fixture 语义不变 | moved test functions | `git diff --color-moved=dimmed-zebra`; compare original function-name set and review non-move hunks |
| B-003 cfg 边界不变 | module-local imports, tests and helpers | `rg -n "#\\[cfg" tests/cli.rs tests/cli`; feature/platform CI |
| B-004 target/filter 兼容 | `tests/cli.rs` mod declarations | `cargo test --test cli home_flag_runs_without_panicking_on_empty_home`; equivalent representative filters |
| B-005 所有文件低于 800 行 | proposed module layout | `find tests/cli -name "*.rs" -print0 | xargs -0 wc -l` and `wc -l tests/cli.rs` |
| B-006 零生产/infra 改动 | scoped diff | `git diff --name-only origin/main...HEAD` contains only `tests/cli.rs` and `tests/cli/*.rs` |
| B-007 所有 gate 通过 | unchanged tests under supported matrices | default/full/MSRV CI plus #238 feature matrix after dependency lands |

## Dependencies And Ordering

- Spec PR 可独立合并，因为它只描述机械重组。
- implementation 受 #238 阻塞：纯 no-default tests 当前在 `origin/main` 已知失败。
- #238 implementation 合并后，GH242 implementation 必须从新的
  `origin/main` 创建并执行 feature matrix；不得在 GH242 中顺带修复 #238。

## Risks And Mitigations

- **风险：** attribute 与测试分离导致平台覆盖变化。**缓解：** 以 attribute + item
  为原子移动单位，并执行三平台 CI。
- **风险：** 模块 import 修复隐藏了生产/断言改动。**缓解：** 允许的非移动 hunk
  仅限 imports、`mod` declarations 和必要私有路径。
- **风险：** test path 前缀影响精确过滤。**缓解：** 保留函数名，并验证
  `cargo test --test cli <函数名>`。
- **风险：** 为减少 imports 过早抽象 helper。**缓解：** 模块自行导入，遵守
  search-first 和三次重复阈值。

## Verification Plan

Focused:

```sh
cargo test --test cli -- --list
cargo test --test cli
cargo test --test cli tmp_flag_scans_rust_targets_under_temp_worktree
cargo test --test cli home_flag_reports_user_tool_safe_caches
cargo test --test cli agent_doctor_json_runs_for_codex
cargo test --test cli clean_dry_run_does_not_delete
cargo test --test cli free_target_met_writes_plan_and_exits_zero
```

File and scope checks:

```sh
find tests/cli -name "*.rs" -print0 | xargs -0 wc -l
wc -l tests/cli.rs
git diff --check
git diff --name-only origin/main...HEAD
```

Repository gate after #238 lands:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
cargo test --no-default-features
cargo test --no-default-features --features tui
cargo test --no-default-features --features graveyard
rustup run 1.95 cargo build --all-targets --all-features
rustup run 1.95 cargo test
```

## Rollback

本变更只移动测试文件。若测试发现、cfg 或过滤行为变化，回滚 implementation
commit 即可；没有生产代码、schema 或数据需要迁移。

## Human Gates

- Spec PR 与 implementation PR 分离。
- #238 implementation 和本 Spec 都进入 `main` 后才开始 GH242 implementation。
- 不自行批准、合并或 force push。
