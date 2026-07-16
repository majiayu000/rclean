# Candidate Size Warning Propagation - Tech Spec

## Linked Artifacts

- GitHub issue: `#240`
- Product spec: `specs/GH240/product.md`
- Tasks: `specs/GH240/tasks.md`
- Route: `write_spec`

## Codebase Context

| Area | Current evidence | Decision |
| --- | --- | --- |
| `src/scan/sizer.rs:83` | `SizeSummary` 只有 candidate/source bytes。 | 增加 sizing warning 输出，不改变 bytes 含义。 |
| `src/scan/sizer.rs:88` | `summarize` 用 rayon 并行计算候选，只收集 `u64`。 | 每个候选返回 bytes + warnings，再按候选顺序归并。 |
| `src/scan/sizer.rs:117` | 候选根 metadata 错误仅 debug 并返回 0。 | 返回 `MetadataError` 与 0。 |
| `src/scan/sizer.rs:139` | 串行 partition 的 `read_dir`/metadata 错误仅 debug。 | 保留累计 bytes 并返回 warning。 |
| `src/scan/sizer.rs:216` | multi-root rayon 和 parallel walker 只返回 bytes。 | 为两条路径增加局部 warning accumulator，归并后稳定排序。 |
| `src/scan/sizer.rs:290` | walkdir 串行路径忽略 walk/metadata 错误。 | 返回部分 bytes 和结构化 warning。 |
| `src/scan/project.rs:63` | project materialization 消费 `SizeSummary`，但没有 warning 输出。 | 返回 project report 与 sizing warnings 的内部结果。 |
| `src/scan/mod.rs:212` | phase 2 只取得 `ProjectReport`。 | 在 serial project loop 中把 sizing warnings 加入顶层 warnings。 |
| `src/model.rs:71` | 已有 `WalkError` 和 `MetadataError` 可序列化类型。 | 复用现有 variants，不改 schema。 |
| `src/output.rs:234` | human renderer 已打印顶层 warnings 和 incomplete 摘要。 | 不新增输出分支，只验证新 warning 到达顶层。 |

## Root Cause

phase 1 为避免重复遍历会剪枝候选子树；phase 2 sizer 因此是候选内部读取错误
的唯一观察者。但 sizer API 只表达 `u64`，错误被降级为 debug 日志后没有通道
回到 `scan::scan` 的顶层 warning accumulator。并行路径也只共享原子 byte
计数，没有 warning accumulator。

## Proposed Design

1. 在 `sizer` 内引入私有 sizing outcome，包含 `bytes: u64` 与
   `warnings: Vec<ScanWarning>`。`SizeSummary` 增加归并后的 warnings。
2. `dir_size`、partition、parallel walker 和 walkdir helper 都返回 outcome，
   成功 bytes 用 saturating addition 合并，错误转换为现有 `WalkError` 或
   `MetadataError`。
3. parallel walker 使用独立的线程安全 warning accumulator；不得与 byte
   atomic 形成嵌套锁。walker 完成后按 warning kind、path、error 的稳定 key
   排序。
4. rayon 候选和 multi-root 计算先生成每个输入位置的 outcome；利用 indexed
   collection 保留候选/root 输入顺序，再 flatten 已排序的局部 warnings。
5. project materialization 返回一个私有组合结果（`ProjectReport` + sizing
   warnings）。`scan::scan` 在已排序的 project loop 内依次扩展顶层 warnings。
6. blocked 候选继续直接产生 0 bytes 和空 warning，不进入文件系统 sizing。
7. 复用 `ScanReport.warnings` 与现有 output renderer；不增加 JSON variant、
   schema version 或新 CLI flag。

## Data Flow

```text
candidate drafts (stable order)
  -> parallel candidate sizing
       -> per-candidate { bytes, warnings }
       -> per-walker warnings sorted by stable key
  -> SizeSummary { candidate_bytes, source_bytes, warnings }
  -> project build output { report, warnings }
  -> scan::scan top-level warnings
  -> existing JSON serializer / human warning summary
```

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 每个 sizing 读取失败成为结构化 warning | `src/scan/sizer.rs`, `src/scan/project.rs`, `src/scan/mod.rs` | unit test for missing candidate root; Unix integration test for unreadable descendant; assert `ScanWarning` path/kind |
| B-002 保留部分 bytes 并继续 scan | sizing outcome merge | unit fixture with readable file plus failing branch asserts readable bytes retained and scan succeeds |
| B-003 正常结果不变 | existing and new sizer parity tests | `cargo test scan::sizer::tests::parallel_walk_matches_serial_walk_for_nested_tree` plus no-warning assertion |
| B-004 四条 sizing 路径语义一致 | partition, parallel walker, rayon roots, root metadata helpers | focused tests force each path and assert bytes/warnings outcome |
| B-005 warning 顺序稳定 | sizer warning stable-key sort and ordered flatten | repeat parallel failure fixture and compare warning vectors across runs |
| B-006 JSON/human output暴露不完整 | top-level `ScanReport.warnings`, existing renderer | Unix CLI test asserts JSON warning; human CLI test asserts `Results may be incomplete` |
| B-007 部分 bytes 仍遵守过滤/消费规则 | project materialization | focused test with `min_size` around retained partial bytes; existing `free` tests remain green |
| B-008 trust model不变 | scoped diff and full safety suites | `cargo test clean::`; `cargo test plan::`; `cargo test --test cross_platform`; full repository gate |

## Cross-Platform Test Strategy

- missing-root metadata failure和 helper outcome 测试在所有平台运行。
- Unix 使用 mode `000` 的不可读后代覆盖真实 permission-denied CLI 路径，并在
  teardown 前恢复权限。
- Windows 不伪造权限语义；它执行 helper、normal parity、编译和完整回归测试。
- implementation 不得加入仅为测试改变生产错误路径的 hook。

## Risks And Mitigations

- **风险：** 并行收集导致 warning 顺序抖动。**缓解：** 局部 stable-key sort，
  外层按 indexed input 顺序 flatten，并用重复运行测试证明。
- **风险：** 新锁拖慢大目录 sizing。**缓解：** 只在错误时 push warning，byte
  热路径继续使用 atomic；性能 benchmark 记录 before/after。
- **风险：** 把 blocked 候选读取错误暴露为 warning 会改变安全输出。
  **缓解：** blocked 分支保持 0 + empty warnings，并添加断言。
- **风险：** 将部分 bytes 误当完整值。**缓解：** 顶层 warning 保留在 JSON 和
  human output；不做估算。

## Verification Plan

Focused correctness:

```sh
cargo test scan::sizer
cargo test scan::tests
cargo test --test cli scan_warning
cargo test --test cross_platform
```

Performance evidence:

```sh
cargo bench --bench scan_throughput -- --sample-size 10
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

若 warning 归并造成错误 bytes、非确定性或明显性能回归，回滚 implementation PR。
没有 schema migration、持久化数据或用户清理动作需要撤销。

## Human Gates

- Spec PR 合并前不开始 implementation。
- implementation PR 涉及 scan output，但不改变 clean/delete trust model；仍需完整
  CI、规格对照、review threads 与 PR gate。
- 不自行批准、合并或 force push。
