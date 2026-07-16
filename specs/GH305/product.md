# Scan JSON Staleness Field Names - Product Spec

## Linked Issue

- GitHub issue: `#305`
- URL: `https://github.com/majiayu000/rclean/issues/305`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `small`

## Summary

修正当前 `README.md` 对 `scan --json` staleness 字段的两处命名错误：report
字段应为 `staleAfterDays`，candidate 字段应为 `stalenessDays`。配置文件
`.rclean.toml` 的 `stale_after_days` 键、Rust 内部字段、历史规格和运行时行为均保持不变。

## Problem

`ScanReport` 与 `Candidate` 使用 `#[serde(rename_all = "camelCase")]`，因此公开 JSON
字段分别是 `staleAfterDays` 与 `stalenessDays`。README 的 Current Status 和 Usage
却把它们写成 `stale_after_days` 与 `staleness_days`。用户按当前文档读取字段时会得到
缺失/null 数据，而 README 后面的 user-rule 章节已经正确使用 `stalenessDays`，形成同一
文档内部的 API 契约冲突。

## Goals

- Current Status 使用公开 JSON 名 `staleAfterDays`。
- Usage 同时准确命名 report 的 `staleAfterDays` 与 candidate 的 `stalenessDays`。
- 明确区分 JSON camelCase 字段和 `.rclean.toml` snake_case 配置键。
- 只修当前用户文档，不改变已经发布的运行时契约。

## Non-Goals

- 不修改 `src/`、serde attribute、JSON shape、schema version、CLI flag、退出码或输出值。
- 不重命名 `.rclean.toml` 的 `stale_after_days` 配置键。
- 不重写 `docs/specs/` 或既有 `specs/GH<number>/` 历史记录。
- 不新增测试框架、依赖、cleanup rule 或 release 工作。

## Behavior Invariants

1. **B-001** README Current Status 必须把 report JSON 字段写为 `staleAfterDays`。
2. **B-002** README Usage 必须把 report/candidate JSON 字段分别写为
   `staleAfterDays`/`stalenessDays`。
3. **B-003** user-rule 配置示例继续使用 `stale_after_days = 60`，不得把 TOML 键改为
   camelCase。
4. **B-004** implementation diff 只修改 `README.md` 中两个错误 token，不顺带重写文案、
   历史规格或生产代码。
5. **B-005** focused docs checks、`git diff --check` 与 `cargo fmt -- --check` 通过；Rust
   build/test 可因精确 docs-only diff 跳过，但 CI 仍作为合并门禁。

## Edge Cases

- `stale_after_days` 同时是合法 TOML 配置键和错误 JSON 名；验证不能粗暴禁止 README
  中所有 snake_case occurrence。
- README 已有一个正确的 `stalenessDays` reference；修复不能删除或改坏它。
- Rust 内部 snake_case 字段不是公开 JSON 名，不属于本 issue。

## Acceptance Criteria

- B-001 至 B-005 在 tech spec/tasks 中完整映射。
- README 两处错误 token 精确替换为公开 camelCase 名。
- `.rclean.toml` 示例与后文正确 `stalenessDays` reference 保持有效。
- Spec PR 与 implementation PR 分离；implementation 从 Spec 合并后的最新
  `origin/main` 开始。
- implementation PR 经独立 review、current-head CI、reviewThreads、签名、merge state
  与 SpecRail gate 后再合并。
