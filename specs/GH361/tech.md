# Interactive Restore And Read-Only Graveyard Filtering - Tech Spec

## Linked Artifacts

- GitHub issue: `#361`
- Product spec: `specs/GH361/product.md`
- Tasks: `specs/GH361/tasks.md`
- Route: `implement`

```specrail-planned-changes
{
  "issue": 361,
  "complete": true,
  "paths": [
    "CHANGELOG.md",
    "README.md",
    "specs/GH361/product.md",
    "specs/GH361/tech.md",
    "specs/GH361/tasks.md",
    "src/clean.rs",
    "src/clean/output.rs",
    "src/clean/selection.rs",
    "src/cli.rs",
    "src/graveyard/mod.rs",
    "src/graveyard/restore.rs",
    "src/main.rs",
    "tests/graveyard_subcommands.rs"
  ],
  "spec_refs": [
    "specs/GH361/product.md",
    "specs/GH361/tech.md",
    "specs/GH361/tasks.md"
  ]
}
```

## Codebase Context

| Area | Current evidence | Decision |
| --- | --- | --- |
| `src/cli.rs:143` | `RestoreArgs.id` 是 required `String`，`--to` 因此隐式只用于单 ID。 | 改为 `Option<String>`，新增 `dry_run`，给 `--to` 显式 `requires = "id"`。 |
| `src/cli.rs:174` | `GraveyardListArgs` 只有 `json`。 | 新增 `older_than: Option<Duration>`，复用 `parse_duration`。 |
| `src/main.rs:216` | restore arm 直接 `restore_by_id`。 | 委托给 feature-gated `graveyard::run_restore`，保持 main 薄。 |
| `src/main.rs:228` | list 分支直接 render `yard.list()`。 | render 前调用一次 `filter_records`，human/JSON 共用结果。 |
| `src/graveyard/store.rs:153` | `restore_by_id` 已实现 target exists、parent、cross-FS 与 manifest rewrite。 | 不修改；actual batch 每项只调用它。 |
| `src/graveyard/manifest.rs:19` | `ManifestRecord` 已含 id/deleted_at/size/path。 | 不改 schema；交互与过滤只读这些字段。 |
| `src/clean/selection.rs:192` | 已有编号/逗号/range/`a` parser。 | 提升为 crate-visible 并复用；restore wrapper 增加 `all` / `q`。 |
| `src/clean/output.rs:41` | `confirm_if_needed` 内嵌 `[y/N]` I/O。 | 抽取 crate-visible `confirm_prompt`，clean 与 restore 共用，clean 文案/行为不变。 |
| `tests/graveyard_subcommands.rs:86` | 已有单 ID round-trip、target exists、list/JSON/gc E2E。 | 在同一专用文件增加非 TTY、dry-run、filter 与 clap 回归。 |

## Design

### 1. CLI shape

```rust
pub struct RestoreArgs {
    pub id: Option<String>,
    #[arg(long, requires = "id")]
    pub to: Option<PathBuf>,
    #[arg(long)]
    pub dry_run: bool,
}

pub struct GraveyardListArgs {
    pub json: bool,
    #[arg(long, value_parser = parse_duration)]
    pub older_than: Option<Duration>,
}
```

不新增 `yes`。无参 actual restore 的确认不可绕过；显式 `--id` 保持现有直接恢复。

### 2. `src/graveyard/restore.rs`

新增 feature-gated CLI workflow module，storage primitive 仍在 `store.rs`：

- `run_restore(args) -> Result<ExitCode, RcleanError>`
- `ensure_interactive_terminal()`
- `interactive_records(yard)`: list + stable newest-first sort
- `select_record_indices(input, count)`: `q`、`all` adapter + 复用
  `clean::parse_selection`
- `print_restore_plan(records, dry_run)`
- `restore_selected(yard, records) -> RestoreBatchResult`
- `print_restore_result(result)`
- `filter_records(records, older_than, now)`

`RestoreBatchResult` 保存三个 closed buckets：

```rust
struct RestoreBatchResult {
    restored: Vec<ManifestRecord>,
    skipped: Vec<(ManifestRecord, String)>,
    failed: Vec<(ManifestRecord, String)>,
}
```

错误分类只匹配现有 `GraveyardError` variants；不修改 underlying safety policy。

### 3. Explicit ID path

- 无 `dry_run`：调用一次现有 `restore_by_id(args.id, args.to)`，保持成功/错误输出。
- `dry_run`：调用 `yard.list()` 并按 id 查 record；不存在返回 `GraveNotFound`；
  只打印 “would attempt”，target 使用 `--to` 或 `original_path`。
- dry-run 不调用新的 target validator，避免产生“预览已通过 safety preflight”的
  错误承诺，也避免触碰本 issue 排除的 symlink/TOCTOU policy。

### 4. Interactive path

1. terminal gate；
2. `yard.list()`，newest-first；
3. 显示 numbered records；
4. 读取一行 selection；`q` / empty → exit 3；
5. 打印 selected plan；
6. dry-run → exit 0；
7. actual → `clean::confirm_prompt`；
8. 逐项 `restore_by_id`，完整分类；
9. summary；任何 skipped/failed → exit 1，否则 exit 0。

snapshot 与实际操作之间的并发变化不被隐藏：`GraveNotFound` 进入 skipped。

### 5. `list --older-than`

避免把超大 `std::time::Duration` 转成可能 overflow 的 chrono duration：

```rust
now.signed_duration_since(record.deleted_at)
    .to_std()
    .is_ok_and(|age| age > older_than)
```

只 filter cloned/input records，保留 manifest 顺序。main 把同一个 filtered vector
交给 table 或 JSON renderer，并基于其 emptiness 计算 exit code。

### 6. Shared interaction helpers

- `clean::selection::parse_selection` 从 `pub(super)` 提升为 `pub(crate)` 并由
  `clean.rs` crate-visible re-export；算法不改。
- 从 `confirm_if_needed` 抽取 `confirm_prompt(prompt, cancelled_message)`；
  clean 继续构造原文案并调用它，restore 构造自己的文案。I/O failure 仍为
  `CleanError`，通过 `RcleanError` 透明传播。

## Data Flow

```text
restore --id
  -> dry-run: list/find -> print only
  -> actual: existing restore_by_id -> existing result/error

restore (no id, TTY only)
  -> list snapshot -> newest-first numbered display -> parse -> confirm
  -> for each selected record: existing restore_by_id
  -> restored/skipped/failed summary + deterministic exit

graveyard list --older-than
  -> yard.list -> filter once -> same Vec to human or JSON -> exit from Vec emptiness
```

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 explicit id and `--to` compatibility | CLI + explicit path | existing round-trip/target tests plus alternate target regression |
| B-002 TTY fail closed | `ensure_interactive_terminal` | non-TTY CLI test asserts target/manifest unchanged |
| B-003 active newest-first list | interactive sort/render | unit fixture with fixed timestamps |
| B-004 selection grammar | `select_record_indices` + reused parser | unit table for number/range/all/a/q/invalid/dedup |
| B-005 cancel/empty exit 3 | interactive runner | injected selection unit test and empty CLI test |
| B-006 mandatory confirmation | shared `confirm_prompt` call ordering | injected workflow test proves no `restore_by_id` before yes |
| B-007 per-item existing primitive | `restore_selected` | code anchor + two-record storage fixture |
| B-008 partial failure classification | `restore_selected` | one valid + one conflict fixture continues and buckets correctly |
| B-009 summary and exit | result renderer/runner | unit result test + batch fixture |
| B-010 zero-write dry-run | explicit/interactive dry-run branches | E2E compares payload, manifest bytes, target/parent before/after |
| B-011 dry-run is selection preview only | wording + no validator | stdout assertion contains `would attempt`; conflict state not claimed successful |
| B-012 older-than semantics | `filter_records` | fixed now/past/future unit test + empty exit 3 CLI |
| B-013 human/JSON same vector | main list branch | paired E2E parses JSON and human ids |
| B-014 forbidden flags | clap surface | negative CLI/help tests |
| B-015 I/O fail explicit | reused helpers + `outln!` | existing pipe/I/O tests and scoped review |

## Risks And Mitigations

- **部分完成：** filesystem operation 无法整体事务化。逐项继续、closed buckets 与
  exit 1 防止前两项成功后第三项失败被隐藏。
- **snapshot stale：** selection 后 manifest 可变化。每项重新走 `restore_by_id`，
  `GraveNotFound` 显式 skipped。
- **dry-run 误导：** 不调用不完整的 duplicated validator，文案限定为
  “would attempt”，不声称可成功。
- **scope creep：** manifest/ActionPlan/store planned paths 均不包含 schema 或
  policy change；`list --plan` 明确排除。
- **rollback：** 回退新 workflow、CLI fields、shared helper extraction、docs/tests；
  没有 migration 或持久化格式要回滚。

## Verification

- `cargo test graveyard::restore::tests`
- `cargo test --test graveyard_subcommands`
- `cargo test default_flow_tests`
- `cargo fmt -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo build --release`
- `rustup run 1.95 cargo build --all-targets --all-features`
- `rustup run 1.95 cargo test`
- `cargo build --no-default-features`
