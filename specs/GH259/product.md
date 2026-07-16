# Non-Repository Git Discovery Fast Path - Product Spec

## Linked Issue

- GitHub issue: `#259`
- URL: `https://github.com/majiayu000/rclean/issues/259`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `medium`

## Summary

为 scan-local `GitCache` 增加保守的 `.git` ancestor marker cache，避免扫描大量非
Git sibling projects 时为每个项目启动一次 `git rev-parse`。真实仓库仍由 Git
命令决定 repo root 和 dirty 状态，任何环境覆盖或文件系统歧义都回退现有命令路径。

## Problem

当前 `GitCache` 能按 repo root 共享 `git status`，但 `non_repos` 只缓存被查询的
精确项目目录。100 个同级非仓库项目因此启动 100 个 `git rev-parse` 进程。

在 `origin/main@44273cb` 的 fresh evidence 中：

- Criterion 100 sibling fixture median：1.018s；
- 同一 fixture 默认 Git checks：0.97s、0.97s、1.01s；
- 同一 fixture `--git-timeout 0`：0.02s、0.02s、0.02s；
- 单个 2048-file candidate Criterion median：20.555ms。

约 49 倍的 default/disabled 差距证明进程扇出是该 scan shape 的主成本，而不是
walker、sizer 或 JSON 输出。

## Goals

- 非仓库 siblings 复用共同 ancestor 的无-marker 结论，不再一项目一 Git 进程。
- 每个尚未查询的 child 先检查自己的 `.git`，再复用 parent 结论。
- 同时识别 `.git` directory、worktree/submodule `.git` file 及其他存在的 marker。
- marker 只决定“是否可跳过 Git”；真实仓库的 root 和 dirty 状态仍由 Git 决定。
- 元数据错误、cache lock poison 和 Git discovery 环境覆盖全部保守回退。
- 保持 dirty worktree `safe → caution`、JSON/report、排序和 ActionPlan 行为不变。

## Non-Goals

- 不修改 Git failure、timeout 或 warning 的现有用户可见语义。
- 不把 filesystem marker 当成 repo root 或 dirty-state 权威。
- 不改变 `--git-timeout 0`、CLI schema、scan output schema 或默认 safety policy。
- 不修改 walker、sizer、rule classification、delete、ActionPlan、symlink、broad-root 或
  protected-path 行为。
- 不引入后台线程、持久化 cache、全局 cache 或新依赖。
- 不更新无关依赖或吸收 PR #235。

## Behavior Invariants

1. **B-001** 对同一无-marker parent 下的多个 sibling projects，第一次查询可建立
   scan-local ancestor 结论；后续 sibling 在检查自己的 `.git` 后必须复用 parent
   结论，不启动 `git rev-parse`。
2. **B-002** parent 的 absent 结论不得遮蔽 child 自己的 marker；新增 sibling
   `.git` directory 必须触发 Git-authoritative discovery。
3. **B-003** `.git` file 与 `.git` directory 均视为 marker；marker 类型不由
   rclean 验证，后续 Git command 负责确认 repo。
4. **B-004** 当 marker 存在时，repo root 仍来自现有
   `git rev-parse --show-toplevel`，dirty 仍来自现有
   `git status --porcelain -uall`；dirty candidate 继续从 safe 降为 caution。
5. **B-005** project 位于 scan root 之下、而 repo marker 位于更高 ancestor 时，
   ancestor lookup 必须继续向上发现该 parent repo。
6. **B-006** 当 child nested repo 与 parent repo 同时存在时，child 自己的 marker
   必须优先，Git 返回的最近 repo root 不得被 parent cache 覆盖。
7. **B-007** `GIT_DIR`、`GIT_WORK_TREE`、`GIT_CEILING_DIRECTORIES` 或
   `GIT_DISCOVERY_ACROSS_FILESYSTEM` 等显式 Git discovery override 存在时，marker
   fast path 必须禁用并使用现有 command discovery。
8. **B-008** `.git` metadata lookup 出现非-NotFound 错误，或 marker cache lock
   poisoned 时，不得缓存/返回 absent；必须回退现有 command path。
9. **B-009** cache 只在单次 `GitCache` 生命周期内存在；重复查询同一目录继续保持
   现有 cache 语义，不新增跨 scan 陈旧状态。
10. **B-010** 同 session 100-sibling fixture 的 after median 至少比 before 快 5 倍，
    且 default before/after report 去除 volatile metadata 后完全相同；所有现有 Git、
    scan、plan 和 safety tests 保持通过。

## Edge Cases

- `.git` 是普通文件、目录、symlink 或其他存在的 entry 时均只作为“需要询问 Git”
  的保守提示，不自行解析内容。
- 首个 sibling 已缓存 parent absent 后，第二个 sibling 仍必须 probe 自己的 marker。
- permission denied、I/O error、路径元数据异常不得等同 NotFound。
- cache lock poison 不能把未知状态变成 non-repo。
- scan 期间新建 repo 的竞态沿用现有 exact-directory cache 生命周期；不得扩大到跨 scan。

## Boundary Checklist

| Boundary | Verdict |
| --- | --- |
| Empty / missing input | N/A：不改变 scan root 输入契约。 |
| Error and failure paths | Covered by B-007/B-008：不确定状态回退 command。 |
| Authorization / permission | Covered by B-008：permission error 不得推断 absent。 |
| Concurrency / race / ordering | Covered by B-001/B-002/B-006/B-009。 |
| Retry / repetition / idempotency | Covered by B-009：cache 仅 scan-local。 |
| Illegal state transitions | Covered by B-008：poison/unknown 不得转成 clean/non-repo。 |
| Compatibility / migration | Covered by B-004/B-010：输出与 safety 行为不变。 |
| Degradation / fallback | Covered by B-007/B-008：明确 command fallback。 |
| Evidence and audit integrity | Covered by B-010：同 fixture benchmark 与 normalized report diff。 |
| Cancellation / interruption / partial completion | N/A：无持久化写入。 |

## Acceptance Criteria

- B-001 至 B-010 在 tech spec 和 tasks 中均有确定性验证映射。
- deterministic tests 覆盖 no-marker siblings、child marker、`.git` file、parent repo、
  nested repo、environment override、metadata error 和 poisoned-cache fallback。
- 100-sibling same-session before/after median 至少 5x，normalized report diff 为空。
- full repository/MSRV/VibeGuard/three-platform gates 通过。
- Spec PR 只包含 `specs/GH259/`，implementation PR 另行创建。
