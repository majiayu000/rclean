# `--no-default-features` Feature Matrix - Tech Spec

## Linked Artifacts

- GitHub issue: `#238`
- Product spec: `specs/GH238/product.md`
- Tasks: `specs/GH238/tasks.md`
- Route: `write_spec`

## Codebase Context

| Area | Current evidence | Decision |
| --- | --- | --- |
| `src/cli.rs:285` | `permanent` 始终存在。 | 保留参数和语义。 |
| `src/cli.rs:286` | `conflicts_with = "graveyard"` 无条件生成。 | 仅在 `graveyard` feature 启用时生成该冲突关系。 |
| `src/cli.rs:289` | `graveyard` 字段受 `#[cfg(feature = "graveyard")]` 控制。 | 保留现有 feature gate。 |
| `src/main.rs:523` | `default_flow_tests` 会构建默认 `clean` 参数图。 | 增加明确的活动 feature 参数图回归测试。 |
| `.github/workflows/ci.yml:35` | CI 只检查 `--no-default-features --features graveyard`。 | 增加无 `graveyard` 的测试组合，并保留其他组合证据。 |
| `docs/specs/v0.1.x-roadmap.md:488` | 规格承诺纯 `--no-default-features` 会编译掉 graveyard CLI。 | 本修复恢复既有契约，不新增产品行为。 |
| `docs/specs/v0.2-best-ux.md:114` | 规格要求 `--no-default-features` 保持工作。 | 作为兼容性验收来源。 |

## Root Cause

Clap derive 会在构建 `clean` 子命令时解析 `conflicts_with` 引用。关闭
`graveyard` 后，被引用参数不存在，因此 debug assertion 在运行时触发。
默认测试和当前 CI 都启用了 `graveyard`，使该关系始终有效并掩盖问题。

## Proposed Design

1. 在 `CleanArgs::permanent` 上使用 feature 条件属性：只有启用
   `graveyard` 时才声明 `conflicts_with = "graveyard"`；`permanent` 字段本身
   始终保留。
2. 保留 `CleanArgs::graveyard` 当前的 `#[cfg(feature = "graveyard")]`，不创建
   哑参数、隐藏别名或运行时占位。
3. 增加一个意图明确的 CLI 参数图测试，对当前编译 feature 下的完整
   `Cli::command()` 执行 Clap debug assertion。该测试在无 `graveyard` 的构建中
   必须能在修复前失败、修复后通过。
4. 扩展 CI feature matrix，至少执行：
   - `cargo test --no-default-features`
   - `cargo test --no-default-features --features tui`
   - `cargo test --no-default-features --features graveyard`
   默认 `cargo test` 继续覆盖 `default`。

## Data And Control Flow

```text
Cargo feature selection
  -> Rust cfg removes or keeps CleanArgs::graveyard
  -> cfg_attr removes or keeps permanent -> graveyard conflict
  -> Clap builds a self-consistent command graph
  -> parser preserves existing clean dispatch and delete-mode behavior
```

没有新的持久化数据、JSON 字段、ActionPlan 字段或迁移。

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 无 `graveyard` 时所有 clean 入口不得 panic | `src/cli.rs`, CLI command-graph unit test | `cargo test --no-default-features cli_graph_is_valid_for_active_features` and `cargo run --no-default-features -- clean --help` exit `0` |
| B-002 无 `graveyard` 时保留 `--permanent` 且不暴露 `--graveyard` | `src/cli.rs` conditional attribute | `cargo run --no-default-features -- clean --help` contains `--permanent` and omits `--graveyard` |
| B-003 有 `graveyard` 时保持互斥 | existing Clap conflict plus regression test | `cargo test --no-default-features --features graveyard` and focused parse assertions for both flags together |
| B-004 四种 feature 组合有效 | CI feature-matrix commands | `cargo test --no-default-features`; `cargo test --no-default-features --features tui`; `cargo test --no-default-features --features graveyard`; `cargo test` |
| B-005 CI 具有真实无 `graveyard` 证据 | `.github/workflows/ci.yml` | `rg -n "cargo test --no-default-features" .github/workflows/ci.yml` plus CI check results on the implementation PR |
| B-006 外部契约和安全语义不变 | scoped diff; default/full gates | `git diff --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test`; `cargo build --release`; MSRV build/test |

## Risks And Mitigations

- **风险：** 条件属性写错会在启用 `graveyard` 时意外取消互斥。
  **缓解：** graveyard-only 和 default 组合都执行冲突回归测试。
- **风险：** 只验证顶层帮助会继续漏掉子命令参数图。
  **缓解：** 直接构建完整 `Cli::command()`，并运行 `clean --help`。
- **风险：** feature matrix 增加 CI 时间。
  **缓解：** 组合数量固定为四个，不引入 powerset 工具或新依赖。

## Verification Plan

Focused feature matrix:

```sh
cargo test --no-default-features
cargo test --no-default-features --features tui
cargo test --no-default-features --features graveyard
cargo test
```

Repository gate:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95 cargo build --all-targets --all-features
rustup run 1.95 cargo test
```

## Rollback

若任一 feature 组合、默认行为或互斥断言回归，回滚 implementation PR 即可。
本变更不包含数据迁移或 schema 变更，回滚不需要用户操作。

## Human Gates

- Spec PR 合并前不开始实现。
- implementation PR 必须通过 CI、规格对照和 PR gate 后才可报告 merge-ready。
- 不在本任务中自行合并或 force push。
