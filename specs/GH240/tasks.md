# GH240 Tasks

## Linked Artifacts

- Issue: `#240`
- Product spec: `specs/GH240/product.md`
- Tech spec: `specs/GH240/tech.md`
- Route after spec approval: `implement`

## Status

`planned` — 等待 Spec PR 人类合并门禁；本文件不代表规格已批准。

## Implementation Tasks

### SP240-T1 — 为 sizing helper 建立 bytes + warnings outcome

- Owner: `implementation`
- Dependencies: merged Spec PR for `#240`
- Covers: B-001, B-002, B-003, B-004
- Change: 让候选根 metadata、串行 partition、parallel walker 和 walkdir helper
  返回部分 bytes 与结构化 warnings；blocked 候选保持 0 + empty warnings。
- Done when: 四条路径在成功和失败 fixture 上产生一致语义，且无 silent debug-only
  downgrade。
- Verify:
  - `cargo test scan::sizer`

### SP240-T2 — 确定性归并并传播到 ScanReport

- Owner: `implementation`
- Dependencies: SP240-T1
- Covers: B-001, B-005, B-006
- Change: 稳定排序并归并并行 warnings，通过 project build output 传递到
  `scan::scan` 顶层 accumulator。
- Done when: 重复并行 fixture 得到完全相同的 warning vector，JSON 和 human
  output 使用现有 renderer 显示 sizing failure。
- Verify:
  - `cargo test scan::sizer`
  - `cargo test --test cli scan_warning`

### SP240-T3 — 增加部分尺寸、过滤和跨平台回归测试

- Owner: `implementation`
- Dependencies: SP240-T1, SP240-T2
- Covers: B-002, B-003, B-004, B-006, B-007, B-008
- Change: 添加 missing-root 全平台测试、Unix permission-denied CLI 测试、部分
  bytes/min-size 测试和正常 parity 断言；不增加生产 test hook。
- Done when: 修复前的真实复现失败，修复后 warning 非空、部分 bytes 正确，既有
  safety/plan/clean tests 保持绿色。
- Verify:
  - `cargo test scan::tests`
  - `cargo test --test cli scan_warning`
  - `cargo test --test cross_platform`
  - `cargo test clean::`
  - `cargo test plan::`

## Verification And Handoff Tasks

### SP240-T4 — 性能与完整 gate

- Owner: `verification`
- Dependencies: SP240-T1, SP240-T2, SP240-T3
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008
- Done when: benchmark 无明显回归，feature/full/MSRV checks、规格对照和 PR gate
  都具有 implementation PR 当前 head 的新鲜证据。
- Verify:
  - `cargo bench --bench scan_throughput -- --sample-size 10`
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95 cargo build --all-targets --all-features`
  - `rustup run 1.95 cargo test`

## Invariant Coverage Audit

- Product invariant set: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008}`
- Task coverage union: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007, B-008}`
- Missing invariants: `none`

## Handoff Notes

- #240 只扩展现有 scan warning 契约，不新增 warning schema。
- implementation 必须从 Spec PR 合并后的最新 `origin/main` 创建。
- 不自行批准、合并或 force push。
