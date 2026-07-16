# Canonical Scan Root Deduplication - Tech Spec

## Linked Artifacts

- GitHub issue: `#253`
- Product spec: `specs/GH253/product.md`
- Tasks: `specs/GH253/tasks.md`
- Route: `write_spec`

```specrail-planned-changes
{
  "issue": 253,
  "complete": true,
  "paths": [
    "src/scan/mod.rs",
    "tests/cli/scan_clean.rs"
  ],
  "spec_refs": [
    "specs/GH253/product.md",
    "specs/GH253/tech.md",
    "specs/GH253/tasks.md"
  ]
}
```

## Codebase Context

| Area | Current evidence | Decision |
| --- | --- | --- |
| `src/scan/mod.rs:151` | `scan(paths, options)` owns root normalization and 整个 scan pipeline。 | 在此处建立唯一的 canonical-root 输入序列。 |
| `src/scan/mod.rs:160` | 每个原始 path 在循环内 canonicalize 后立即扫描。 | 把 canonicalize/dedup 前置到 walker 之前。 |
| `src/scan/mod.rs:167` | 每次循环无条件 push `ScanReport.roots`。 | 只对第一次出现的 canonical root push。 |
| `src/scan/mod.rs:183` | `walk_parallel` 会为每个循环 root 执行完整遍历。 | 迭代去重后的 roots，消除重复工作而非只修输出。 |
| `src/model.rs:112` | `ScanReport.roots/projects/summary` 是公开 JSON 输出。 | 不改 schema，只阻止重复元素进入。 |
| `src/plan/io.rs:15` | ActionPlan 直接复制 report roots/projects/selected。 | 不改 plan writer；上游唯一性自动传播。 |
| `tests/cli/scan_clean.rs:178` | 已有 scan→write plan→dry-run 端到端测试。 | 在同一测试模块增加 duplicate/alias/unique-root 回归。 |

## Root Cause

`scan()` 同时承担 root canonicalization 与 per-root 执行，但没有 canonical-root
identity set。每个输入在 canonicalize 成功后立刻进入 `roots.push`、walker、sizer
和 project 汇总，所以后处理无法区分“两个真实 roots”与“同一 root 的重复输入”。
ActionPlan writer 忠实复制 report，进一步把重复项持久化。

## Proposed Design

1. 在 `scan()` 的 per-root 工作前，用私有 helper 或等价局部逻辑遍历输入 paths，
   对每个 path 执行现有 `canonicalize` 与 `CanonicalizeRoot` 映射。
2. 使用标准库 `HashSet<PathBuf>` 记录已见 canonical roots，并用 `Vec<PathBuf>` 保存
   第一次插入成功的 roots；不增加依赖。
3. canonicalization 对所有输入保持 fail-fast：任何唯一或重复拼写输入若自身无法
   canonicalize，仍返回指向该原始 path 的现有错误。
4. 原 per-root pipeline 改为迭代有序 unique vector。`roots` 字符串、user rules、
   ignore matcher、walker、git cache、sizer 和 project materialization 均只执行一次。
5. 不对 path prefix、ancestor/descendant 或 inode 内容做额外等价判断；identity
   仅为 `PathBuf` canonical 结果相等。
6. `ScanReport`、ActionPlan、summary 与 clean code 不改；唯一性通过既有数据流自然
   传播。

## Data Flow

```text
raw paths
  -> canonicalize each input (existing error mapping)
  -> ordered exact-dedup { HashSet seen + Vec first_occurrences }
  -> per-unique-root ignore/walk/git/size/project pipeline
  -> ScanReport roots/projects/summary
  -> existing ActionPlan writer and dry-run
```

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 exact canonical roots scan once | `src/scan/mod.rs` root normalization | CLI regression passes same path twice and asserts one project/candidate |
| B-002 first occurrence order | ordered unique vector | JSON assertion compares `roots` with first-occurrence order across duplicate inputs |
| B-003 alias spellings dedup | canonical path identity | CLI test passes `path` and `path/.` and asserts one root |
| B-004 no duplicate internal pipeline work | dedup before per-root loop | code review anchor plus progress/summary assertion: one project scanned, one candidate |
| B-005 report and ActionPlan unique | existing report→plan flow | write plan, parse JSON arrays, then dry-run asserts `Plan: 1 candidates` |
| B-006 ancestor/descendant preserved | equality-only seen set | focused negative test passes two distinct canonical roots and asserts both roots remain |
| B-007 canonicalize failure remains explicit | normalization error mapping | existing missing-root behavior plus focused test asserts failure names invalid input |
| B-008 unique-root behavior unchanged | scoped diff and repository regression | existing scan/plan/clean tests plus full repository gate |

## Test Strategy

- 在 `tests/cli/scan_clean.rs` 建立临时 Node marker 与 `node_modules` candidate。
- 同一测试覆盖字面重复和 `path/.` alias；解析 scan JSON/ActionPlan，避免只用字符串
  次数猜测结构。
- 使用单独 fixture 覆盖 ancestor/descendant distinct roots，防止实现演变成 prefix
  collapse。
- 复用现有 missing-root、plan dry-run 与 single-root tests；不加入生产 test hook。

## Risks And Mitigations

- **风险：** 使用 sort/dedup 改变 root 顺序。**缓解：** HashSet 只做 membership，
  Vec 保留第一次出现顺序并显式测试。
- **风险：** 误合并重叠 roots。**缓解：** 只比较 canonical `PathBuf` equality，增加
  ancestor/descendant 负例。
- **风险：** 先扫描再去重只修饰输出。**缓解：** dedup helper 必须位于
  `walk_parallel` 和其他 per-root 工作之前，summary 断言证明只扫描一次。
- **风险：** dedup 掩盖坏输入。**缓解：** 每个原始输入先 canonicalize，错误保持
  fail-fast 且测试错误路径。

## Verification Plan

Focused correctness:

```sh
cargo test --test cli duplicate_canonical_scan_roots
cargo test --test cli scan_write_plan_then_clean_plan_dry_run
cargo test scan::tests
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

若 exact dedup 改变 unique-root 行为、错误语义或 root 顺序，回滚 implementation
commit 即可。没有 schema、持久化格式、迁移或已删除数据需要恢复。

## Human Gates

- Spec PR 与 implementation PR 分离；Spec 合并前不改生产代码。
- implementation 从 Spec 合并后的最新 `origin/main` 创建。
- PR 仅在 current-head CI、review threads、merge state 与用户 standing merge 授权都
  有新鲜证据时合并；禁止 force push。
