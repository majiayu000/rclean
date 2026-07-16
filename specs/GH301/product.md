# Node and Python Classifier Test Matrix - Product Spec

## Linked Issue

- GitHub issue: `#301`
- URL: `https://github.com/majiayu000/rclean/issues/301`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

在现有 `tests/rules/project_artifacts.rs` 中补齐 Node 与 Python 项目 artifact classifier
的端到端契约矩阵。测试通过真实
`rclean scan --json --min-size 0 --include-blocked` 解析候选，精确断言
`ruleId` 与 `safety`，覆盖所有正向分支、marker 缺失拒绝路径，以及 Python plain
`venv` 的 blocked 分支；生产规则与安全行为保持不变。

## Problem

最新 `origin/main@63a8ca8` 的 `cargo llvm-cov --all-features --summary-only` 显示总行
覆盖率为 82.88%，但 `src/rules/node.rs` 仅 38.57%，`src/rules/python.rs` 仅 26.76%。
当前 project-artifact E2E 主要覆盖 Node `build`、Python `__pycache__` 和几个跨生态优先级
样本，没有以一个完整矩阵锁定两个 classifier 的精确 rule/safety 契约。

缺少这些测试时，未来的 dispatch、marker 或规则重构可能把 safe cache 改成 caution、
把 generic directory 错报为可清理，或丢失 plain `venv` 的 blocked 保证，而现有 suite
仍然通过。

## Goals

- 覆盖 Node 的 `node_modules`、`.next`、`.turbo`、`.vite`、`.parcel-cache`、
  `build`、`dist`、`out` 全部规则分支。
- 证明缺少 `package.json` 时相同目录名不产生 Node rule。
- 覆盖 Python valid `.venv`、valid plain `venv`、`__pycache__`、pytest、mypy、ruff、
  tox 规则与 safety。
- 证明 Python marker 存在但 `.venv` 无 virtualenv marker 时不分类，而 plain `venv`
  无 virtualenv marker 时保持 blocked。
- 证明缺少 Python project marker 时相同目录名不产生 Python rule。
- 解析 JSON 并比较精确 rule/safety 集合，不依赖宽松 substring 或候选顺序。

## Non-Goals

- 不修改 `src/`、classifier、marker helper、rule id、category、safety、warning 或 restore hint。
- 不新增 cleanup rule、candidate name、CLI flag、schema、依赖、测试框架或 coverage CI 阈值。
- 不改变扫描、选择、ActionPlan、clean/delete、graveyard、symlink、broad-root 或
  protected-path 策略。
- 不为提高数字而测试不可控的 TUI、OS 磁盘 API 或故障注入分支。
- 不吸收 Dependabot PR #235 或私有 security advisory 工作。

## Behavior Invariants

1. **B-001** 带 `package.json` 的 fixture 必须精确产生 8 个 Node rule：
   `node.node_modules`、`node.next`、`node.turbo`、`node.vite`、`node.parcel`、
   `node.build`、`node.dist`、`node.out`。
2. **B-002** Node 前五个规则保持 `safe`，三个 generic build-output 规则保持 `caution`；
   无 `package.json` fixture 不得产生任何 `node.*` rule。
3. **B-003** 带 Python marker 与 virtualenv marker 的 `.venv`/`venv` 分别产生
   `python.venv_dot`/`python.venv_plain`，且均为 `safe`。
4. **B-004** `python.pycache`、`python.pytest`、`python.mypy`、`python.ruff` 保持
   `safe`；`python.tox` 保持 `caution`。
5. **B-005** Python project 中无 virtualenv marker 的 `.venv` 不产生候选；同条件下
   plain `venv` 产生 `python.venv_plain` 且为 `blocked`。
6. **B-006** 无 Python project marker fixture 不得产生任何 `python.*` rule。
7. **B-007** 新测试必须使用 `--include-blocked` 从结构化 JSON 构建
   ruleId-to-safety 集合并精确比较，不能用只证明某个 substring 出现的断言，也不能
   依赖 candidates 输出顺序。
8. **B-008** implementation 只修改 `tests/rules/project_artifacts.rs`，不使用 ignore、
   sleep、wall-clock assertion 或测试弱化，并通过 focused/full stable、MSRV、VibeGuard、
   SpecRail 与三平台 CI 门禁。

## Edge Cases

- 一个项目同时有多个 Node candidate，测试必须证明每个 rule 只出现一次。
- `.venv` 和 plain `venv` 的缺 marker 行为故意不同，测试不能把二者合并成一个预期。
- Python marker 可以使用现有 `pyproject.toml`；本问题不枚举四种等价 project marker。
- 扫描顶层可能包含多个 fixture project；断言按 rule id 集合比较而不是依赖 project 顺序。
- blocked candidate 只有显式 `--include-blocked` 才进入报告；测试必须传入该 flag，且不得
  改变默认过滤或 min-size 语义。

## Boundary Checklist

| Boundary | Verdict |
| --- | --- |
| Empty / missing input | Covered by B-002/B-006 marker-missing fixtures. |
| Error and failure paths | Covered by B-005 invalid virtualenv marker behavior. |
| Authorization / permission | N/A：测试不改变权限。 |
| Concurrency / race / ordering | Covered by B-007：集合比较不依赖输出顺序。 |
| Retry / repetition / idempotency | Covered by exact unique rule map assertions. |
| Illegal state transitions | N/A：无运行时状态变更。 |
| Compatibility / migration | Covered by B-008：测试 target 与生产 API 不变。 |
| Degradation / fallback | Covered by B-002/B-005/B-006：拒绝/blocked 不得静默变 safe。 |
| Evidence and audit integrity | Covered by B-007 和 coverage 前后证据。 |
| Cancellation / partial completion | N/A：无长事务或持久化。 |

## Acceptance Criteria

- B-001 至 B-008 在 tech spec/tasks 中有完整映射。
- focused `rules` target 新测试全部通过，并精确证明 Node/Python matrix。
- post-change coverage report 证明两个 classifier 的所有可达 match arm 被执行；不在 CI
  添加固定 coverage 百分比 assertion。
- implementation diff 只有 `tests/rules/project_artifacts.rs`，生产代码零变更。
- Spec PR 与 implementation PR 分离；实现从 Spec 合并后的最新 `origin/main` 开始。
- full stable/release/MSRV/VibeGuard/CI/PR gates 通过。
