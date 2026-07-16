# Graceful Closed-Pipe Output - Product Spec

## Linked Issue

- GitHub issue: `#281`
- URL: `https://github.com/majiayu000/rclean/issues/281`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `high`

## Summary

让 rclean 的非交互 stdout 在下游读取者提前关闭时按 CLI 管道语义结束，而不是由
`println!` 触发 panic、向 stderr 泄漏 backtrace 提示并以 101 退出。同时保留命令自身已经
确定的 0/1/3/4 结果、普通 stdout 内容、错误可见性和全部清理安全顺序。

## Problem

当前 `main@38c0699` 的共享输出路径大量直接调用 `println!`。Rust 标准库明确规定该宏在
stdout 写入失败时 panic。真实执行 `rclean rules | head -n 1` 与
`rclean doctor | head -n 1` 均产生 `failed printing to stdout: Broken pipe`，rclean 退出 101，
而下游 `head` 正常退出 0。

这不是单个命令的格式问题。report、JSON、free、stamp、watch 和 clean summary 等非交互
输出共享同一失败模式；若只修两个复现命令，其他公开入口仍会随机 panic。

## Goals

- 所有非交互 stdout 写入使用可失败的 I/O，不再依赖会 panic 的 `println!`。
- 只把 `std::io::ErrorKind::BrokenPipe` 识别为下游主动终止；其他 stdout 错误仍可见且 exit 1。
- 命令输出前已经确定的语义退出码在断管时保持不变。
- 删除前输出遇到断管时立即停止，绝不因错误处理而继续进入 destructive path。
- 删除完成后的断管不覆盖已经计算出的 clean 成功/失败状态。
- 普通 human/JSON 输出、字段、换行、排序和 stderr 内容保持现状。
- 用真实关闭的匿名 pipe 做跨平台 E2E 回归，不依赖 shell、`head` 或竞态。

## Non-Goals

- 不改变候选扫描、分类、排序、选择或大小计算。
- 不改变 ActionPlan schema、写入、replay、delete mode 或 clean 结果。
- 不改变 symlink、broad-root、protected-path、TOCTOU、graveyard 或 restore 行为。
- 不吞掉 permission denied、storage full、invalid handle、JSON serialization 或其他错误。
- 不捕获/匹配 panic 文本，不新增全局 panic hook。
- 不启用 nightly `on_broken_pipe`，不改变进程级 SIGPIPE disposition。
- 不改变交互选择/TUI 的终端渲染，也不处理 stderr 被关闭的场景。

## Behavior Invariants

1. **B-001** `rules` 与 `doctor` 连接到已关闭 stdout pipe 时不 panic，不输出 panic/backtrace，
   并返回各自已经确定的语义退出码。
2. **B-002** 其他非交互 human 与 JSON output paths 使用同一 fallible stdout primitive；不得
   为每个命令复制 BrokenPipe 字符串判断。
3. **B-003** 只有 `ErrorKind::BrokenPipe` 走 quiet termination；任何其他 stdout I/O 错误
   仍通过 `RcleanError::OutputIo` 报错并 exit 1。
4. **B-004** JSON 必须先完整 serialization，再写 stdout；断管不得产生 panic，正常打开的
   stdout 仍是原有单一 JSON document 和 schema。
5. **B-005** 输出前已经确定 0/1/3/4 的命令在断管时保留该状态，不统一重写为 0。
6. **B-006** clean 在 scan table、plan 或 confirmation 之前的 stdout 断管必须返回且不执行
   删除；不得把输出失败转化为继续执行的授权。
7. **B-007** clean 已完成执行后再发生断管，必须保留由 `result.failed` 决定的 0/1 状态；
   不得把实际删除失败静默改成成功。
8. **B-008** free/stamp 等先写计划或 stamp、后输出的命令保留原操作顺序；断管不回滚已完成
   的文件操作，也不伪造不同的语义状态。
9. **B-009** watch 初始输出断管时不进入长期 watcher/poll loop；后续 diff 输出断管时正常
   终止 watch，而不是继续后台运行。
10. **B-010** stdout 正常打开时，现有 human/JSON E2E assertions 全部保持通过；不得借机改文案、
    字段、空结果或换行契约。
11. **B-011** E2E 使用 `std::io::pipe()` 并在 spawn 前关闭 reader，确定性覆盖至少两个独立
    命令；测试在 Rust 1.95 和三类 CI OS 上运行。
12. **B-012** 实现不改变 trust model；focused、stable、release、exact MSRV、VibeGuard 和
    current-head PR gates 全部通过。

## Acceptance Criteria

- B-001 至 B-012 在 tech spec 与 tasks 中完整映射。
- 修复前 E2E 对 `rules` 和 `doctor` 均观察到非成功/101 与 panic stderr，修复后通过。
- 单元测试证明 BrokenPipe 与 PermissionDenied 等非-BrokenPipe I/O 的分流不同。
- clean 的 pre-delete stop 与 post-delete semantic status 有直接测试或现有行为证据。
- 普通输出相关的现有 CLI suites 全量通过。
- README 记录管道行为，architecture 记录 fallible stdout boundary。
- Spec PR 只包含 `specs/GH281/`；implementation PR 独立创建。
