# README Published Rule Scope And Safety Catalog - Product Spec

## Linked Issue

- GitHub issue: `#246`
- URL: `https://github.com/majiayu000/rclean/issues/246`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

补齐 README 漏记的 12 个已发布规则，修正现有 Go module download safety 漂移，
并按 `--home`、`--tmp`、`--system` 和显式 path scan 准确说明发现范围、路径、
safety 与恢复方式，使 “full rule list” 不再与当前 rule catalog/classifiers 冲突。

## Problem

`src/rules/catalog.rs` 已公开 12 个 README 表格没有的 rule ids。部分只在 prose
中零散出现，部分完全没有用户文档。现有 `go.module_download_cache` 行还标为
`safe`，但 classifier 已是 `caution` 并要求 native `go clean -modcache`。尤其是
macOS temp/system 规则，如果被错误归入 `--home` 或 `--tmp`，会让用户误解扫描
边界和是否可被普通 cleanup 选择。

## Goals

- 为 12 个遗漏规则提供结构化、可搜索的 README 表项。
- 把 `go.module_cache` 与 `go.module_download_cache` 都准确记录为 `caution`，并
  说明 native cleanup 和 redownload/offline-build 风险。
- 明确每个规则由 `--home`、`--tmp`、`--system` 或显式 path scan 发现。
- 准确记录 path pattern、safety、恢复方式和必要的进程/管理员提示。
- 保持 README “完整列表”措辞与实际覆盖范围一致。

## Non-Goals

- 不增加、删除或修改任何 cleanup rule。
- 不改变 home/tmp/system root expansion、classification、selection 或 deletion。
- 不把显式 path rule 扩展到默认 `--tmp`，也不扩大 macOS allowlist。
- 不新增 CI、生成器、依赖或 README 自动同步脚本。
- 不重写 project-level ecosystem 或 AI model-store 表格。

## Behavior Invariants

1. **B-001** README 必须以结构化表项记录以下 12 个 rule ids，且每个 id 恰好
   出现在其 canonical rule-directory entry 中：`go.module_cache`、
   `app.lark_cache`、`macos.chrome_code_sign_clone`、`macos.remem_dry_run_tmp`、
   `agent.tmp_worktree`、`apple.wallpaper_aerial_videos`、`apple.idleassetsd`、
   `chrome.opt_guide_model`、`app.lark_update`、`macos.geod_map_tiles`、
   `macos.mediaanalysisd_cache`、`macos.mediaanalysisd_tmp`。
2. **B-002** discovery scope 必须与代码一致：8 个 home rules、2 个 tmp rules、
   `apple.idleassetsd` 的 system scope，以及
   `macos.chrome_code_sign_clone` 的显式 path scan；不得声称后者的 `X` 目录由默认
   `--tmp` 自动扫描。
3. **B-003** safety 必须保持当前分类：`go.module_cache` 和现有
   `go.module_download_cache` 都是 `caution` 且走 `go clean -modcache`；
   `app.lark_cache`、
   `macos.chrome_code_sign_clone`、`macos.remem_dry_run_tmp` 为 `safe`；
   `agent.tmp_worktree`、wallpaper/Chrome model/Lark update 和三个 user-service
   caches 为 `caution`；`apple.idleassetsd` 为 `report-only`、需要管理员权限且
   永不被 `clean` 选择。
4. **B-004** 每个表项的 path pattern、restore hint 和必要 warning 必须来自当前
   classifier/catalog；tmp whole-worktree 必须说明 marker 与 `--include-caution`，
   process-sensitive rules 必须提示关闭相关进程或服务。
5. **B-005** “full rule list” 的措辞必须准确限定其覆盖，或通过 scope-specific
   分组使已发布 whole-machine/explicit rules 都可从同一目录发现；不得把 project
   rule examples 冒充完整 rule-id 清单。
6. **B-006** implementation 只允许修改 `README.md`；Rust、测试、CI、依赖、规则
   catalog 和 trust-model 行为保持不变。
7. **B-007** 12 个 id 的 README 覆盖必须用确定性搜索验证；现有 global table 的
   safety 必须全量对照 classifier，确认并修正所有冲突；scope/safety 必须逐项对照
   当前 `origin/main` 的 classifier、root expansion 和 tests，不得凭记忆填写。

## Scope Matrix

| Discovery scope | Rules |
| --- | --- |
| `--home` | `go.module_cache`, `app.lark_cache`, `apple.wallpaper_aerial_videos`, `chrome.opt_guide_model`, `app.lark_update`, `macos.geod_map_tiles`, `macos.mediaanalysisd_cache`, `macos.mediaanalysisd_tmp` |
| `--tmp` | `macos.remem_dry_run_tmp`, `agent.tmp_worktree` |
| `--system` | `apple.idleassetsd` |
| Explicit path scan | `macos.chrome_code_sign_clone` |

## Edge Cases

- `std::env::temp_dir()` on macOS normally points into a `T` directory;这不等于扫描相邻
  的 `X` directory。
- `agent.tmp_worktree` 是整个临时 worktree 的 caution candidate，不是其中的普通
  `target/` safe candidate。
- `go.module_cache` 必须使用 native `go clean -modcache` cleanup 路径，不能沿用
  `go.module_download_cache` 当前 README 中错误的 `safe` 表述。
- `apple.idleassetsd` 的 path 可被报告不代表 rclean 会执行 sudo 或选择删除。

## Boundary Checklist

| Boundary | Verdict |
| --- | --- |
| Empty / missing input | N/A：文档目录变更。 |
| Error and failure paths | Covered by B-003/B-004：warning、report-only 和 native-tool 限制保持可见。 |
| Authorization / permission | Covered by B-003：system rule 明确需要管理员权限且永不自动选择。 |
| Concurrency / race / ordering | Covered by B-004：process-sensitive rules 保留关闭进程提示。 |
| Retry / repetition / idempotency | N/A：无运行时行为。 |
| Illegal state transitions | N/A：不改变 state machine。 |
| Compatibility / migration | Covered by B-002/B-005/B-006。 |
| Degradation / fallback | Covered by B-006：不改变发现或 cleanup fallback。 |
| Evidence and audit integrity | Covered by B-001/B-007。 |
| Cancellation / interruption / partial completion | N/A：无事务。 |

## Acceptance Criteria

- B-001 至 B-007 在 tech spec 和 tasks 中均有验证映射。
- 12 个遗漏 id 均有 scope/path/safety/restore evidence。
- 两个 Go module-cache rows 均为 caution/native cleanup，现有 safety 表无已知冲突。
- 没有把 system、tmp 或 explicit-path 边界扩大成更广的自动扫描承诺。
- Spec PR 只包含 `specs/GH246/`；implementation PR 另行创建且只改 README。
