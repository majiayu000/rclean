# Scan Output Grouping (parts b, c) - Tech Spec

## Linked Artifacts

- GitHub issue: `#358`
- Product spec: `specs/GH358/product.md`
- Tasks: `specs/GH358/tasks.md`
- Route: `implement`

```specrail-planned-changes
{
  "issue": 358,
  "complete": true,
  "paths": [
    "CHANGELOG.md",
    "specs/GH358/product.md",
    "specs/GH358/tech.md",
    "specs/GH358/tasks.md",
    "src/output.rs",
    "tests/cli/scan_clean.rs"
  ],
  "spec_refs": [
    "specs/GH358/product.md",
    "specs/GH358/tech.md",
    "specs/GH358/tasks.md"
  ]
}
```

## Codebase Context

| Area | Current evidence | Decision |
| --- | --- | --- |
| `src/output.rs:6` | `print_json` 直接序列化 `ScanReport`。 | 不修改，保证 JSON 顺序与 schema 不受影响。 |
| `src/output.rs:147` | `print_table` 独立负责 human scan rendering。 | 所有行为变更限制在该输出层及私有 helper。 |
| `src/output.rs:214` | 外层按 `report.projects`、内层按 `project.candidates` 输出。 | 保留 project 作为 app/parent 分组边界。 |
| `src/output.rs:226` | `Kind` 当前始终取 `project.kind`。 | 已知 kind 优先，否则从当前 candidate 的 `rule_id` 派生。 |
| `src/output.rs:545` | 已有纯展示 helper `truncate_path`。 | 沿用私有纯函数风格，不增加 model 字段。 |
| `tests/cli/scan_clean.rs:31` | 已有 human table 与 JSON CLI 测试。 | 增加 Kind、视觉分组、排序与 JSON 不变回归。 |

## Root Cause

human table 把 project-level `kind` 直接复制到每一行，但 global cache 的 project
通常只是共享 parent，没有 marker，因此 candidate 明明由精确 rule 分类，展示仍丢失
生态信息。

同一 project 的 candidates 已天然连续，但 renderer 重复 project path 且不建立
display order，导致现有数据分组没有转化成可读的视觉层次。问题不在 candidate model，
不需要 aggregation 或 selection 改造。

## Proposed Design

### 1. Candidate-level Kind

增加私有纯函数：

```rust
fn candidate_kind<'a>(project_kind: &'a str, rule_id: &'a str) -> &'a str {
    if project_kind != "Unknown" {
        project_kind
    } else {
        rule_id.split_once('.').map_or(rule_id, |(prefix, _)| {
            if prefix.is_empty() { rule_id } else { prefix }
        })
    }
}
```

row rendering 传入当前 candidate 的 `rule_id`。已知 project kind 始终优先；
无点号或空前缀 rule id 原样显示。内置 rule id 均为非空，helper 不新增静默
fallback。

### 2. Display-only stable ordering

增加 `sorted_candidates(project) -> Vec<&Candidate>`（或等价私有 helper）：

1. 收集 `project.candidates.iter()` 的引用；
2. 用稳定排序按 `Reverse(candidate.bytes)` 排列；
3. 相同 size 保持原输入顺序。

该 vector 只由 `print_table` 消费。`ScanReport` 自身、JSON、clean、tui、free 与
ActionPlan 均继续读取原 `project.candidates`。

### 3. Project continuation rows

遍历排序后的引用时：

- index `0` 的 Project 单元格显示既有 `short_path + truncate_path`；
- index `> 0` 的 Project 单元格为空字符串；
- Kind、Candidate、Category、Size、Junk、Safety、Risk、Stale 与 Reason 每行仍
  从该 candidate 独立读取。

不插入 summary row，不删除 candidate row，也不新增可选择 parent。Project path
从“每行重复”变为“组首 header cell”，取得视觉分组但不改变行身份。

## Data Flow

```text
ScanReport (unchanged)
  -> print_json -> original projects/candidates order (unchanged)
  -> print_table
       -> each ProjectReport is one visual group
       -> stable size-desc Vec<&Candidate>
       -> candidate_kind(project.kind, candidate.rule_id)
       -> one human data row per Candidate
```

## Product-to-Test Mapping

| Behavior invariant | Implementation area | Verification |
| --- | --- | --- |
| B-001 known project kind wins | `candidate_kind` | unit test `candidate_kind_keeps_known_project_kind` |
| B-002 dotted rule prefix shown | `candidate_kind` | unit cases for `go.build_cache` and `editor.vscode_cache`; CLI `--home` fixture |
| B-003 no-dot rule id preserved | `candidate_kind` | unit test with `custom` |
| B-004 one row per candidate | `print_table` loop | CLI fixture parses table data rows and finds both candidate names |
| B-005 stable size-desc order | `sorted_candidates` | unit tests for descending sizes and equal-size input order; CLI order assertion |
| B-006 project path only on group head | indexed row rendering | CLI fixture inspects Project cells for first/continuation rows |
| B-007 JSON unchanged | `print_json` remains untouched | CLI JSON fixture asserts candidate count/names；unit test proves display sort does not mutate report order |
| B-008 selection/ActionPlan untouched | output-only diff boundary | planned-path review plus existing `clean`/`plan`/`tui` full tests |
| B-009 stdout errors remain explicit | existing `outln!` calls | scoped diff confirms no error handling change; full test gate |

## Test Strategy

- Unit：直接验证 Kind 派生和 reference render order，不通过字符串猜测排序。
- CLI human：临时 Node project 同时创建两个不同大小的 candidates，定位 table
  header 之后的数据行，验证每个 candidate 独立出现、较大项先出现、project path
  只在第一条 candidate 行显示。
- CLI home：临时 `HOME` 下创建精确 global cache anchor，验证 `Unknown` project
  以 ecosystem Kind 输出。
- CLI JSON：同一多 candidate fixture 解析 JSON，验证 candidate 数量与原扫描顺序
  未被 human display sort 改写。

## Risks And Mitigations

- **风险：** display sort 意外 mutate report。**缓解：** 只排序 `Vec<&Candidate>`，
  并用 JSON 顺序测试锁定原 model。
- **风险：** continuation row 被误解为漏 project。**缓解：** 每个 group 首行始终
  显示 path；单 candidate project 不受影响；CLI 测试覆盖新 group restart。
- **风险：** 把 B-c1 演变成 B-c2。**缓解：** B-004 要求一 candidate 一行，
  planned paths 不包含 selection/model/plan 模块。
- **回滚：** 回退 `src/output.rs` render helper/loop 与对应 tests/changelog 即可；
  没有 schema、持久化或 migration 状态。

## Verification

- `cargo test output::tests`
- `cargo test --test cli scan_table`
- `cargo fmt -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo build --release`
- `rustup run 1.95 cargo build --all-targets --all-features`
- `rustup run 1.95 cargo test`
