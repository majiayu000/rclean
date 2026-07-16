# README Published Rule Scope And Safety Catalog - Tech Spec

## Linked Artifacts

- GitHub issue: `#246`
- Product spec: `specs/GH246/product.md`
- Tasks: `specs/GH246/tasks.md`
- Route: `write_spec`

## Codebase Context

| Evidence | Current fact | Documentation decision |
| --- | --- | --- |
| `src/rules/catalog.rs` | 12 listed ids exist in the public `rclean rules` catalog but not README tables. | Add canonical README entries without modifying catalog. |
| `src/cli.rs::home_toolchain_paths` | `go`, macOS Library/Application Support/Containers roots are `--home` inputs. | Group the 8 home-discovered missing rules under home scope. |
| `src/cli.rs::tmp_workspace_paths` | default tmp roots include `temp_dir`, `/private/tmp`, `/tmp`; no sibling `X` expansion. | Document two T/tmp rules under `--tmp`; keep code-sign clone explicit. |
| `src/rules/macos_system.rs::system_scan_paths` | only exact idleassetsd anchor is returned for `--system`. | Document report-only + administrator boundary. |
| `src/rules/go.rs` | module-cache root and download subtree are both caution and use native `go clean -modcache`. | Add the missing root row and correct the existing download row from safe to caution. |
| `tests/macos_scan_cli.rs`, `tests/cli.rs`, `tests/system_cli.rs` | fresh behavior fixtures prove home/tmp/system output and safety. | Use tests as evidence, do not edit them. |

## Proposed README Layout

Keep the current project-level and AI model-store sections. In the whole-machine area:

1. Add or expose a discovery-scope column/grouping for home rules.
2. Add missing home entries with exact or pattern paths, safety and restore hint; correct the
   existing `go.module_download_cache` safety/native-cleanup text.
3. Add a compact temporary/explicit macOS rules group:
   - `--tmp`: `macos.remem_dry_run_tmp`, `agent.tmp_worktree`;
   - explicit scan: `macos.chrome_code_sign_clone`.
4. Add the `--system` report-only `apple.idleassetsd` entry.
5. Correct the surrounding “full rule list” wording so the directory scope is explicit.

## Product-to-Verification Mapping

| Behavior invariant | Documentation area | Verification |
| --- | --- | --- |
| B-001 all 12 ids | README rule directory | fixed expected-id loop requires every literal id |
| B-002 discovery scope | home/tmp/system/explicit groups | compare `home_toolchain_paths`, `tmp_workspace_paths`, `system_scan_paths` and macOS classifier anchors |
| B-003 safety | Safety column/notes | compare all existing global-table safety cells with classifier/tests; correct both Go module-cache rows |
| B-004 path/restore/warnings | each new row | compare classifier `reasons`, `warnings`, `restore_hint`, catalog hint and deletion native-tool mapping |
| B-005 full-list wording | whole-machine intro and rule headings | review scope language against existing project/AI sections |
| B-006 README-only | implementation diff | `git diff --name-only origin/main...HEAD` equals `README.md` |
| B-007 evidence integrity | all new/existing rows | run expected-id, full safety comparison, code-symbol and focused test-source searches |

## Expected-ID Check

```sh
for id in \
  go.module_cache app.lark_cache macos.chrome_code_sign_clone \
  macos.remem_dry_run_tmp agent.tmp_worktree apple.wallpaper_aerial_videos \
  apple.idleassetsd chrome.opt_guide_model app.lark_update \
  macos.geod_map_tiles macos.mediaanalysisd_cache macos.mediaanalysisd_tmp
do
  rg -F "\`$id\`" README.md
done
```

This proves literal coverage. Reviewer inspection of the four scope groups remains required;
an id appearing only in prose does not satisfy B-001.

The implementation review must also repeat the full global-table safety comparison used during
triage. A spot check of only the Go row does not satisfy B-007.

## Risks And Mitigations

- **风险：** 把 `X` code-sign clone 写成默认 `--tmp` 范围。**缓解：** 对照
  `tmp_workspace_paths`，将它标为 explicit path scan。
- **风险：** 复制旧 README 的 Go safety 错误。**缓解：** 以 `src/rules/go.rs` 的
  `Safety::Caution` 和 native cleanup 为准。
- **风险：** 把 report-only 写成 caution/safe。**缓解：** system row 必须同时包含
  report-only、requires administrator 和 never selected。
- **风险：** 文档表格过大难以扫描。**缓解：** 以 discovery scope 分组，保持每个
  rule 一行并避免重复 prose。

## Verification Plan

Focused fact checks:

```sh
rg -n 'go\.module_cache|Safety::Caution|go clean -modcache' src/rules/go.rs src/clean/deletion.rs
rg -n '^\| `go\.module_(download_)?cache` .*\| caution \|' README.md
rg -n 'app\.lark_cache|macos\.chrome_code_sign_clone|macos\.remem_dry_run_tmp|agent\.tmp_worktree' src/rules src/cli.rs
rg -n 'apple\.wallpaper_aerial_videos|apple\.idleassetsd|chrome\.opt_guide_model|app\.lark_update|macos\.geod_map_tiles|macos\.mediaanalysisd_' src/rules/macos_system.rs tests
rg -n 'home_toolchain_paths|tmp_workspace_paths|system_scan_paths' src/cli.rs src/rules/macos_system.rs
```

File and scope checks:

```sh
git diff --check
test "$(git diff --name-only origin/main...HEAD)" = "README.md"
```

Repository docs gate:

```sh
cargo fmt -- --check
```

Rust build/test are not required locally because B-006 permits only README Markdown changes.
Existing GitHub CI still supplies cross-platform repository evidence.

## Rollback

本变更只添加/整理 README rule entries 并修正一个现有 safety cell。事实或 scope 有误时回滚 implementation
commit 即可；没有代码、schema、数据或安全策略迁移。

## Human Gates

- Spec PR 与 implementation PR 分离。
- Spec PR 人工合并后，implementation 从当时最新 `origin/main` 创建。
- 不自行批准、合并或 force push。
