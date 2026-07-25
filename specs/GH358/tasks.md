# GH358 Tasks

## Linked Artifacts

- Issue: `#358`
- Product spec: `specs/GH358/product.md`
- Tech spec: `specs/GH358/tech.md`
- Route: `implement`

## Status

`approved_for_implementation` — maintainer 已选择 B-b1 + B-c1；B-c2 true
aggregation 明确 deferred。

## Implementation Tasks

### SP358-T1 — 锁定 Kind 与 display grouping 回归

- Owner: `implementation`
- Dependencies: maintainer B-b1 + B-c1 decision
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007
- Change: 在 `src/output.rs` unit tests 与 `tests/cli/scan_clean.rs` 增加 known/unknown
  Kind、size order、equal-size stability、project continuation rows、JSON 原顺序测试。
- Done when: tests 明确区分 B-c1 与被 deferred 的 summary collapse；human table
  每个 candidate 仍有独立行，JSON order 未变化。
- Verify:
  - `cargo test output::tests`
  - `cargo test --test cli scan_table`

### SP358-T2 — 实现 candidate-level Kind

- Owner: `implementation`
- Dependencies: SP358-T1
- Covers: B-001, B-002, B-003, B-009
- Change: 在 `src/output.rs` 增加私有 `candidate_kind` helper，并让 human row
  rendering 使用当前 candidate 的 `rule_id` 派生 Unknown project 的生态。
- Done when: known project kind 不变，global cache 显示生态，无点号 rule id 原样显示，
  `outln!` 错误传播不变。
- Verify:
  - `cargo test output::tests::candidate_kind`
  - `cargo test --test cli scan_table_shows_ecosystem_kind_for_global_cache`

### SP358-T3 — 实现一行一 candidate 的视觉分组

- Owner: `implementation`
- Dependencies: SP358-T1
- Covers: B-004, B-005, B-006, B-007, B-008, B-009
- Change: 在 `src/output.rs` 为每个 project 建立 stable size-desc reference order；
  仅首行打印 Project path，continuation rows 留空；不修改 model 或 JSON。
- Done when: human table 视觉归组且所有 candidate 行仍存在；JSON、selection、
  ActionPlan 与 trust-model 模块无 diff。
- Verify:
  - `cargo test output::tests::sorted_candidates`
  - `cargo test --test cli scan_table_groups_project_rows_without_collapsing_candidates`
  - `cargo test --test cli scan_json_keeps_all_candidates_after_human_grouping`

### SP358-T4 — 文档与变更记录

- Owner: `implementation`
- Dependencies: SP358-T2, SP358-T3
- Covers: B-004, B-007, B-008
- Change: 更新 `CHANGELOG.md`，明确 human-only、一行一 candidate、JSON/selection/
  ActionPlan 不变；保持 `specs/GH358/` 与实现一致。
- Done when: changelog 不使用 aggregation/collapse 等会误导 reviewer 的表述。
- Verify:
  - `rg -n "Kind|candidate|JSON|ActionPlan" CHANGELOG.md specs/GH358`

## Verification And Handoff Tasks

### SP358-T5 — 完整 repository 与 PR gate

- Owner: `verification`
- Dependencies: SP358-T1, SP358-T2, SP358-T3, SP358-T4
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009
- Done when: focused/full/MSRV checks、spec-vs-implementation 对照、review 与 CI
  均绑定 PR 当前 head；PR 只包含 planned paths；不执行 merge。
- Verify:
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95 cargo build --all-targets --all-features`
  - `rustup run 1.95 cargo test`

## Invariant Coverage Audit

- Product invariant set:
  `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009}`
- Task coverage union:
  `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008, B-009}`
- Missing invariants: `none`

## Handoff Notes

- B-c1 only：不允许 `"N items"` summary、父行 selection 或 candidate collapse。
- implementation 只允许修改 manifest 中的 planned paths。
- PR 使用 `Refs #358`；#358 继续保留 B-c2 deferred decision，不用 `Closes #358`。
- maintainer review 与 merge 是独立 gate；本任务的“开工”不授权 merge。
