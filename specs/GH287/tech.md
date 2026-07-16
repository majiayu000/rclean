# 小项目根 Marker 快照 - Tech Spec

## Linked Artifacts

- GitHub issue: `#287`
- Product spec: `specs/GH287/product.md`
- Tasks: `specs/GH287/tasks.md`
- Route: `write_spec`

## Codebase Context

| Area | Current evidence | Decision |
| --- | --- | --- |
| `src/rules/project.rs::detect_project_kind` | 15 exact stats + repeated kind checks | 保留 targeted implementation，前置 bounded snapshot fast path。 |
| `markers::has_prefixed_marker` | 每个 prefix 单独 `read_dir`，entry type 不受限 | snapshot 用 `entry_names` 保持该语义。 |
| `markers::has_marker_with_extension` | .NET extension 匹配任意 entry path | snapshot 不收紧为 file-only。 |
| `markers::has_marker` | exact marker 用 `Path::is_file()`，跟随 file symlink | snapshot 用独立 `file_names` 保持该语义。 |
| `markers::package_mentions` | 每个 dependency 重新读 package raw | snapshot path 读一次 raw，复用相同 substring 判断。 |
| `benches/scan_throughput.rs` | 已有 100-small、one-huge、many-wide | 新增 1,000-small marker-heavy shape，保留原三项。 |

## Proposed Internal Design

在 `src/rules/project.rs` 内增加私有常量和 snapshot：

```rust
const PROJECT_ROOT_SNAPSHOT_LIMIT: usize = 64;

struct ProjectRootSnapshot {
    entry_names: Vec<OsString>,
    file_names: HashSet<OsString>,
}
```

不要求上述精确容器类型，但必须保留两组语义：

- `entry_names` 保存 threshold 内所有 entry 的原始 `OsString` name，供 prefix/extension
  checks 使用；
- `file_names` 只保存 `entry.path().is_file()` 为真的 exact markers。regular entry 可先用
  `DirEntry::file_type()` 避免额外 stat；symlink 必须用 `Path::is_file()` 跟随确认；
- non-UTF-8 name 原样计数/存储，不使用 lossy string；UTF-8 prefix/extension match 对它返回
  false。

## Snapshot Algorithm

1. `fs::read_dir(dir)` 失败 => `None`（选择 targeted fallback）。
2. 顺序读取 entries；任何 `Err` => `None`。
3. 每个成功 entry 先计数。读到第 65 个时立即 `None`；partial state 被丢弃。
4. 获取 `file_type`；错误 => `None`。regular file 加入 `file_names`；symlink 仅在
   `entry.path().is_file()` 时加入；directory/other 不加入。
5. 所有原始 names 加入 `entry_names`。
6. 完整结束且 `<=64` => `Some(snapshot)`。

`detect_project_kind` 变成 router：

```rust
match ProjectRootSnapshot::read(dir) {
    Some(snapshot) => detect_project_kind_from_snapshot(dir, &snapshot),
    None => detect_project_kind_targeted(dir),
}
```

`detect_project_kind_targeted` 是当前函数体的等价提取，既是 fallback 也是 focused test 的
reference oracle。不得顺手修改 `src/rules/markers.rs` 或 classifiers。

Snapshot detector 按固定 marker array 生成 markers。若 `package.json` 是 exact file marker，
只调用一次 `fs::read_to_string`，缓存 `Option<String>`；Next/Vite 使用与
`package_mentions` 相同的 quoted substring needle，config prefix 仍优先。其他 kind 使用
snapshot exact/prefix/extension predicates，并严格保留 B-006 priority。

## Product-to-Test Mapping

| Invariant | Implementation area | Deterministic verification |
| --- | --- | --- |
| B-001 bounded small root | snapshot reader | <=64 fixture returns snapshot-path result and benchmark removes repeated probes |
| B-002 65+ fallback | count-before-accept | exactly 64 vs 65 fixture; both equal targeted reference |
| B-003 ambiguity fallback | `Result/Option` routing | missing-root/read failure reference equality; code review covers entry/file-type errors |
| B-004 exact file semantics | `file_names` | regular file, directory, symlink-to-file and broken symlink fixtures |
| B-005 entry-name semantics | `entry_names` | config-prefix directory and `.csproj` directory preserve current matches |
| B-006 order/priority | snapshot detector | table-driven all-kind and mixed-marker tests compare targeted reference exactly |
| B-007 one package read | cached raw | Next/Vite/package-only/invalid UTF-8 package fixtures; source review shows one read site |
| B-008 non-UTF-8 | raw `OsString` | Unix in-memory name predicate/count test; never `to_string_lossy` |
| B-009 edge equivalence | both detectors | table-driven `(kind, markers)` equality |
| B-010 report equivalence | release binaries | normalized same-fixture JSON diff removing only `scannedAt` |
| B-011 performance | manual + Criterion | 15-run timing table and four Criterion shapes |

## Planned Changes Manifest

| Path | Change |
| --- | --- |
| `src/rules/project.rs` | Extract targeted reference, add bounded snapshot detector and focused semantic tests. |
| `benches/scan_throughput.rs` | Add fixed 1,000-small-project marker-heavy benchmark shape; preserve existing shapes. |

No change is permitted in `markers.rs`, classifiers, walker, scan report builder, dependencies,
schema, CLI, safety, delete, ActionPlan or private security artifacts.

## Safety And Error Notes

- Snapshot is an optimization hint. Any ambiguity selects the exact current detector, not Unknown.
- Do not swallow `ReadDir` entry/file-type errors into an apparently complete snapshot.
- Exact marker semantics must follow symlinks only to determine file-ness, matching `Path::is_file`;
  no candidate symlink policy changes.
- Preserve current prefix/extension entry-name behavior even where it appears permissive; behavioral
  tightening requires a separate issue and safety review.
- Raw non-UTF-8 names are never converted lossily and never removed from the threshold count.
- No state survives the function call, so filesystem mutations are no staler than current checks.

## Benchmark Design

Add `marker_heavy_small_projects_json` with 1,000 sibling Node projects, each containing
`package.json`, one source file and a tiny `node_modules` candidate. Fixture construction stays
outside timed closure. Keep sample size and all existing benchmark functions unchanged.

Acceptance uses the fixed `/tmp` fixture from issue discovery and both release binaries on the same
machine. Warm each revision, run 15 measurements, report sorted range/median, and require >=15%
median improvement. Re-run existing 100-small, one-huge and many-wide Criterion shapes before/after;
each after point estimate must be <=1.10x baseline. Do not add timing assertions to tests/CI.

## Output Equivalence

Run baseline and implementation release binaries against the same fixed fixture without mutating it
between runs. Remove only top-level `scannedAt`, sort JSON keys, and require an empty diff. Marker
vectors, kind strings and all candidate/report fields remain in the comparison.

## Verification Plan

Focused:

```sh
cargo test rules::project::tests
cargo test scan::tests::scan_sizes_one_large_candidate_and_many_small_projects_deterministically
cargo bench --bench scan_throughput -- --noplot
```

Scope and repository gates:

```sh
git diff --check
git diff --name-only origin/main...HEAD
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
```

## Rollback

Revert the implementation commit. `detect_project_kind` then resumes the current targeted I/O path;
there is no schema, dependency, cache, persistence or migration state.

## Human Gates

- Spec and implementation remain separate PRs.
- Merge only after current-head CI, unresolved-thread, merge-state, output-diff, performance and
  spec-vs-implementation evidence is green.
- Private security advisories remain outside this public issue/PR and are not modified here.
- The user has provided standing merge authorization for this optimization run; never force push.
