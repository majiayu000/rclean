# Deterministic Fake Docker Report Tests - Product Spec

## Linked Issue

- GitHub issue: `#323`
- URL: `https://github.com/majiayu000/rclean/issues/323`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

消除三个 fake Docker report integration tests 在多进程并发负载下把 permission-denied、success 与
oversized-output 结果误报为 5 秒 timeout 的 flake。三个非-timeout fixtures 显式使用 30 秒 test
invocation timeout；专门的 timeout test 继续使用 1 秒。生产默认 timeout、fake scripts、assertions
与所有用户可见 Docker behavior 保持不变。

## Problem

`tests/docker_report_cli.rs` 中三个非-timeout fake Docker tests 没有传 `--timeout`，因此继承生产
默认 5 秒：

- `docker_report_permission_denied_is_explicit`
- `docker_report_success_is_report_only_and_never_prunes`
- `docker_report_large_output_is_explicit_error`

在 `origin/main@965cce9` 上，普通 targeted tests 可连续 10/10 通过，但 12 路并发 targeted run
稳定复现 6/12 失败。保留完整输出的并发批次在原第 55、105、176 行分别观察到
`permissionDenied`/success/`error` 被 `timedOut`、`timeoutMs: 5000` 取代。每个 test 使用独立
`TempDir`，失败发生在等待即时 fake shell command 时；根因是非-timeout fixture 误用生产默认
deadline 作为共享 runner 的测试调度预算。

GitHub issue/PR 搜索只找到 Docker feature #159/#184，没有相同 test-reliability work。专门的
`docker_report_timeout_is_explicit` 已显式使用 `--timeout 1s`，不是本问题。

## Goals

- 三个非-timeout fake report invocations 使用有界 30 秒 test-only headroom。
- permission-denied、success/report-only、oversized-output 与 no-prune contracts 保持原样。
- 专用 timeout test 继续真实覆盖 1 秒 timeout 与 `timeoutMs == 1000`。
- 12 路并发 targeted stress 不再被生产默认 5 秒误报干扰。

## Non-Goals

- 不修改 `src/docker.rs` 的 5 秒 `DEFAULT_TIMEOUT` 或 production command lifecycle。
- 不序列化测试、不增加 mutex、不设置全局 `RUST_TEST_THREADS`。
- 不修改 fake scripts、fixtures、expected statuses 或 assertion predicates。
- 不增加 retry、sleep、skip、依赖、workflow 或 CI concurrency hack。
- 不修改 Docker report/doctor CLI、JSON schema、安全分类或 pruning policy。

## Behavior Invariants

1. **B-001** permission-denied、success/report-only、oversized-output 三个 fake report commands 各自
   精确增加 `--timeout 30s`；不得修改其他 command arguments。
2. **B-002** `docker_report_timeout_is_explicit` 保留 `--timeout 1s`、`sleep 5` fixture、
   `timedOut` 与 `timeoutMs == 1000` assertions。
3. **B-003** production `DEFAULT_TIMEOUT: Duration = Duration::from_secs(5)` 与全部 `src/` 文件不变。
4. **B-004** 三个 fake scripts、permission-denied/status、resource IDs、selected=false、no-prune 与
   output-limit assertions 全部保持不变。
5. **B-005** implementation diff 只允许修改 `tests/docker_report_cli.rs`，且只包含三个 args
   arrays 的 timeout additions。
6. **B-006** focused tests 连续 10 轮、三轮 12 路并发 stress 与默认 full suite 通过。
7. **B-007** full stable/release、精确 Rust 1.95.0、VibeGuard、SpecRail、独立 review 与
   current-head cross-platform CI/PR gates 全绿。

## Edge Cases

- 30 秒是 test invocation 的有界 scheduling headroom，不改变生产默认或 timeout classification。
- 大输出 fixture 在高负载下可能接近 5 秒；其目标是 64 KiB bound，不应被无关 timeout 抢先。
- timeout fixture 必须保持 1 秒，不能随正常 fixtures 一起提高。
- 跨进程并发是复现条件，test-binary 内 mutex 不能解决多个 cargo test processes 的调度竞争。
- 若实现时命令布局漂移，停止并刷新三个 exact sites，不按旧行号盲改。

## Acceptance Criteria

- B-001..B-007 在 tech/tasks 完整映射。
- Spec 与 implementation PR 分离；实现从 Spec 合并后的最新 main 开始。
- implementation 是单文件、三处 timeout additions，无 assertion/test/production weakening。
- focused 10 轮、三轮 12-way stress、full stable/MSRV 与 guards 通过。
- current-head CI、签名、reviewThreads、merge state、独立 review 与 SpecRail required gate 全绿后合并。
