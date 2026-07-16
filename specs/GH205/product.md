# Deterministic Fake Go Subprocess Tests - Product Spec

## Linked Issue

- GitHub issue: `#205`
- URL: `https://github.com/majiayu000/rclean/issues/205`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

消除三个 fake Go subprocess 单元测试在完整测试套件或共享 runner 高负载下的低频竞态。
测试必须继续覆盖成功、非零退出和 timeout 三种真实进程结果，但不得互相争抢 CPU/进程调度，
也不得让 5 秒测试专用 deadline 把正常的非零退出误报成 timeout。生产 native-tool timeout、
process lifecycle、Go module-cache cleanup 和所有用户可见行为保持不变。

## Problem

PR #212 把 success/nonzero fixture 的测试 timeout 从 1 秒提高到 5 秒，但三个 fake Go
tests 仍由 Rust test harness 并发运行，其中 timeout fixture 使用 CPU-bound loop。2026-07-16：

- README-only PR #307 的 Ubuntu CI 在
  `fake_go_modcache_clean_timeout_is_explicit_failure` 失败，同一 head 重跑通过；
- 本地重复完整 `rclean` binary suite 第 3 轮在
  `fake_go_modcache_clean_nonzero_is_explicit_failure` 失败，整轮耗时 5.01 秒且错误不是预期
  nonzero-exit shape；
- 仅运行三个 focused tests 连续 200 轮通过，说明触发条件是完整 suite/host contention，
  不是确定性的 cleanup regression。

旧修复只增加 deadline，没有消除三个 fixture 之间的并发干扰，也没有把实际错误写入 assertion
failure，因此 CI 只能看到缺少 `timed out`/`exited`，无法看到真实 lifecycle error。

## Goals

- 三个 fake Go subprocess tests 在同一 test binary 内互斥执行。
- success/nonzero fixture 使用有界但显著高于 5 秒的测试专用 deadline。
- timeout fixture 继续独立证明显式 `timed out` 错误。
- 所有原有路径、参数、环境变量、退出状态、stderr 和 timeout 断言保持或增强。
- 未来失败必须在 assertion message 中显示实际错误。

## Non-Goals

- 不修改生产 `GO_CLEAN_MODCACHE_TIMEOUT`、`PIP_CACHE_PURGE_TIMEOUT` 或
  `run_native_tool`。
- 不修改 cleanup selection、delete mode、ActionPlan、graveyard、symlink、broad-root、
  protected path 或审计行为。
- 不降低、删除或跳过现有断言；不设置全局 `RUST_TEST_THREADS`。
- 不增加依赖、workflow retry、CI concurrency hack、公开 API 或用户文档。
- 不顺带拆分 800 行以内的 `src/clean/deletion.rs`。

## Behavior Invariants

1. **B-001** 三个 fake Go subprocess tests 必须共享 test-only mutex，且 lock poisoning 必须
   显式返回测试错误，不得 `unwrap`/静默继续。
2. **B-002** success/nonzero tests 使用 30 秒 test-only timeout；生产 60 秒 timeout 常量保持
   完全不变。
3. **B-003** timeout test 继续使用 50ms deadline、CPU-bound fake program，并断言路径上下文和
   `timed out`；它必须持有同一 mutex。
4. **B-004** success test 继续断言 `clean`、`-modcache`、`GOMODCACHE`；nonzero test 继续断言
   wrapper context、path、`exited` 与 `permission denied`。
5. **B-005** nonzero/timeout string assertions 必须在失败时打印完整 observed error，不能放宽
   expected substring。
6. **B-006** implementation diff 仅修改 `src/clean/deletion.rs` 的 `#[cfg(test)]` module；无生产
   code、dependency、workflow、fixture platform behavior 或 trust-model change。
7. **B-007** focused stress、完整 stable/release、精确 Rust 1.95.0、VibeGuard、SpecRail、独立
   review 与 cross-platform CI gates 通过。

## Edge Cases

- mutex 被某次 panic poison 时，后续 test 必须报告清晰错误而不是 panic on `unwrap`。
- Windows `.cmd` 和 Unix shell fixture 必须走同一串行化契约。
- timeout fixture 的 50ms 是被测 error path，不能跟随 normal fixture 一起提高到 30 秒。
- 增加 headroom 不能变成无限等待；30 秒只存在于 test module。
- focused group 本身可能在旧代码上连续通过，因此 merge 证据必须同时包含历史 CI/本地复现、
  stress repetition 和完整 suite gates，不能把单次 green 当作 root-cause proof。

## Acceptance Criteria

- B-001 至 B-007 在 tech spec/tasks 中完整映射。
- Spec PR 与 implementation PR 分离，implementation 从 Spec 合并后的最新
  `origin/main` 开始。
- implementation 只有 test-module diff，并保留全部原断言语义。
- focused fake-go group 连续 100 轮通过，完整 binary suite 连续 10 轮通过。
- full stable/release、exact MSRV、VibeGuard、current-head CI、签名、reviewThreads、merge
  state、独立 review 与 SpecRail required gate 全绿后才可合并。
