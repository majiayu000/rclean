# 小项目根 Marker 快照 - Product Spec

## Linked Issue

- GitHub issue: `#287`
- URL: `https://github.com/majiayu000/rclean/issues/287`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `medium`

## Summary

`detect_project_kind` 对小型项目根只做一次有界目录快照，复用 exact marker、config prefix、
.NET extension 和 `package.json` 信息，减少每项目重复 metadata I/O。大目录或任何枚举歧义
完整回退现有 targeted checks，保持 marker 顺序、kind priority 和输出不变。

## Problem

当前每生成一个项目报告，`detect_project_kind` 都会：

- 对 15 个已知 marker 分别调用 `is_file()`；
- 为 kind 判断再次检查部分相同 marker；
- Node 项目分别枚举 `next.config.*` 和 `vite.config.*`，并可能两次读取
  `package.json`；
- .NET 判断再次枚举根目录查找 `sln/csproj/fsproj` extension。

在 `origin/main@8f60e21` 的 fresh evidence 中：

- GH284 后 100-small Criterion point estimate：26.009ms；
- 固定 1,000 个 sibling Node 项目 fixture、Git disabled、15 次 warmed release runs：
  154.381–244.520ms，median 202.458ms；
- 静态路径确认每项目至少 15 次 targeted marker stat，之后才进入重复 kind checks。

## Goals

- project root entry 数不超过 64 时，只枚举一次并建立 scan-call-local snapshot。
- snapshot 分开保存 `file_names` 与 `entry_names`：exact marker 只认现有 `is_file`
  语义；prefix/extension 保持现有“任意 entry name”语义。
- marker 输出仍按固定 marker catalog 顺序，不受 filesystem enumeration 顺序影响。
- Node snapshot path 只读取一次 `package.json`，同时服务 Next.js/Vite dependency fallback。
- root 超过 64 entries、`read_dir`/entry/file-type 出错时，完整使用现有 targeted path。
- 保持 project-kind priority、markers、JSON/table、candidate、安全和删除行为不变。
- 增加 1,000-small-project durable benchmark shape。

## Non-Goals

- 不修改 candidate prefilter、30 个 classifiers、rule catalog 或 marker helper 的公共行为。
- 不把 snapshot 接入 phase-1 walker，不增加跨项目或跨 scan cache。
- 不改变 candidate classification、safety tiers、delete、ActionPlan、symlink/broad-root/
  protected-path gates。
- 不把大项目根完整读入内存，也不根据 benchmark 写 wall-clock CI assertion。
- 不新增依赖、schema、CLI flag 或用户配置。
- 不修改依赖版本或吸收 PR #235。

## Behavior Invariants

1. **B-001** 可完整读取且 entry 数 `<=64` 的 root 使用一次 bounded snapshot；snapshot
   只在当前 `detect_project_kind` 调用内存在。
2. **B-002** 第 65 个 entry 一出现就放弃 partial snapshot，并对该项目完整执行现有
   targeted detector；不得混合 partial 与 fallback 结论。
3. **B-003** `read_dir`、任一 entry 或所需 file-type 读取失败时完整 fallback；未知状态
   不得被缓存为 marker absent。
4. **B-004** exact marker 继续要求 `Path::is_file()`；regular file 和指向 file 的 symlink
   算 marker，directory/断链 symlink 不算。
5. **B-005** config prefix 与 `.sln/.csproj/.fsproj` extension 保持现有 entry-name 语义；
   不因 snapshot 擅自增加 file-only 条件。
6. **B-006** markers vector 仍按 15 个 marker 的静态顺序输出，project kind priority 仍为
   Next.js→Vite→Node→Rust→Python→Go→iOS→.NET→Ruby→Maven→Gradle→Flutter/Dart→Unknown。
7. **B-007** snapshot Node path 最多读取一次 `package.json`；config prefix 优先级和当前
   substring dependency detection 保持不变，读取/UTF-8 失败等同当前“不包含 dependency”。
8. **B-008** non-UTF-8 entry 计入 64-entry bound，但不能被 lossy conversion 变成 marker、
   prefix 或 extension evidence。
9. **B-009** 不存在 root、空 root、刚好 64、65+ entries、目录 marker、symlink marker 和
   mixed-language root 均与 targeted reference 返回完全相同 `(kind, markers)`。
10. **B-010** 同 fixture before/after normalized scan JSON 除顶层 `scannedAt` 外为空 diff；
    candidate、bytes、activity、risk、warnings、summary 与排序均不变。
11. **B-011** 同 session 1,000-project median 至少改善 15%；现有 100-small、one-huge 与
    many-wide Criterion point estimate 均不得回退超过 10%。

## Edge Cases

- 根目录不存在或无权限：snapshot 失败并使用现有 targeted detector，通常返回 Unknown。
- 前 64 entries 都可读但第 65 个出现：丢弃整个 snapshot 后重跑 targeted detector。
- entry enumeration 或 file type 在竞态中失败：完整 fallback，不输出 partial markers。
- `package.json` 是 symlink-to-file：仍是 exact marker，并按现有 read-to-string 语义读取。
- config prefix 或 .NET extension 是 directory：保持当前 entry-name match 行为。
- non-UTF-8 entry 不参与 UTF-8 name matching，但仍消耗 bound，防止无界枚举。
- filesystem entry 顺序随机：markers 和 kind 仍确定性输出。

## Boundary Checklist

| Boundary | Verdict |
| --- | --- |
| Empty / missing input | B-009 覆盖 empty/missing root。 |
| Error and failure paths | B-002/B-003 覆盖 threshold 和 I/O ambiguity fallback。 |
| Authorization / permission | permission/read errors 不推断 marker absent。 |
| Concurrency / race / ordering | B-003/B-006 覆盖 entry race 和确定性顺序。 |
| Retry / repetition / idempotency | B-001/B-008：snapshot call-local，无陈旧 cache。 |
| Illegal state transitions | B-002 禁止 partial snapshot 与 fallback 混合。 |
| Compatibility / migration | B-004 至 B-010 保持输出/语义，无迁移。 |
| Degradation / fallback | 大 root 或歧义完整走现有 detector。 |
| Evidence and audit integrity | B-010/B-011 要求 normalized diff 与同 session benchmark。 |
| Cancellation / interruption / partial completion | snapshot 未完成不对外产生 partial report。 |

## Acceptance Criteria

- B-001 至 B-011 在 tech spec 和 tasks 中均有确定性映射。
- tests 覆盖所有 kind priority、marker order、empty/missing、64/65 boundary、read fallback、
  file/directory/symlink marker、prefix/extension entry 和 non-UTF-8 entry。
- 1,000-project median 改善 >=15%，三个现有 Criterion shapes 各自回退 <=10%。
- normalized before/after JSON diff 为空。
- full stable/MSRV/VibeGuard/three-platform gates 通过。
- Spec PR 只包含 `specs/GH287/`；implementation PR 另行创建。
