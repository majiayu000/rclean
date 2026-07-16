# Release Input Dry-Run Coverage - Product Spec

## Linked Issue

- GitHub issue: `#271`
- URL: `https://github.com/majiayu000/rclean/issues/271`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `medium`

## Summary

让真正的 release-input PR 触发 Release workflow，并让 PR dry-run 校验当前
`rclean-cli` Cargo version 对应的非空 changelog 章节。PR 与 tag 必须复用相同的 package
version source 和 release-note extractor，避免历史 `0.2.0` 或重复 AWK 逻辑掩盖缺口。

## Problem

当前 `pull_request.paths` 只包含 `.github/workflows/release.yml`。实际 release PR #213 只改
`Cargo.toml`、`Cargo.lock` 和 `CHANGELOG.md`，因此没有任何 Release workflow check。

即使 workflow 被其他文件触发，`release notes dry run` 仍调用
`extract_release_notes "0.2.0"`。未来 bump 到 `0.3.0` 时，旧的非空 `0.2.0` 章节会让
dry-run 通过，即使当前 `0.3.0` 章节缺失。PR 与 tag job 还各自维护一份相同 AWK 提取逻辑。

## Goals

- release manifest、lockfile、changelog、workflow 或 release scripts 变化时触发 PR dry-run。
- 唯一 package-version helper 从 Cargo metadata 返回当前 `rclean-cli` version，并 fail closed。
- tag verifier、contract tests 和 PR changelog dry-run 复用该 helper。
- PR 与 tag 使用同一个 release-note extractor。
- 当前版本章节缺失或内容为空时明确失败；历史章节不得满足当前版本检查。
- 保留五目标 build dry-run，并保持 draft release/Homebrew 在 PR 上跳过。

## Non-Goals

- 不自动 bump Cargo version、编辑 changelog、创建 tag 或发布 release/package/formula。
- 不改变 changelog heading 格式、release artifact 命名、checksum 或 Homebrew 模板。
- 不改变 GH268 的 tag 与 package version 精确相等契约。
- 不修改 Rust runtime、CLI、scan、clean、ActionPlan、删除或路径安全行为。
- 不顺带修改 workflow permissions、Action 版本或 `TAP_PUSH_TOKEN` 策略。

## Behavior Invariants

1. **B-001** PR paths 至少覆盖 `.github/workflows/release.yml`、`.github/scripts/**`、
   `Cargo.toml`、`Cargo.lock` 和 `CHANGELOG.md`。
2. **B-002** package-version helper 只接受 Cargo metadata 中唯一的 `rclean-cli` package，
   输出其 raw version；工具、metadata 或唯一性失败时非零退出且无 fallback。
3. **B-003** tag verifier 与 contract tests 复用 package-version helper，不保留复制的
   `cargo metadata | jq` package-selection 实现。
4. **B-004** 单一 release-note extractor 按精确 `## <version>` heading 提取到下一个
   `## ` 或 EOF；missing/empty section 明确失败。
5. **B-005** PR dry-run 从 package-version helper 得到当前版本，并用共享 extractor 校验
   `CHANGELOG.md`；不得硬编码 `0.2.0` 或选择任意历史版本。
6. **B-006** tag release job 继续从已通过 GH268 preflight 的 tag 得到版本，但改用同一个
   extractor 生成 `release-notes.md`，draft release body 行为不变。
7. **B-007** contract suite 覆盖 tag exact/invalid/mismatch 与 notes current/missing/empty，
   不编辑 manifest、创建 tag 或访问 GitHub。
8. **B-008** 五目标 matrix、`preflight -> build -> release -> bump-tap` 依赖链与 PR 上两个
   external-effect job 的 SKIPPED 状态保持不变。
9. **B-009** implementation 仅修改 release workflow/scripts；stable/MSRV/VibeGuard、11 个
   required SUCCESS 与 2 个 expected SKIPPED current-head gates 全部通过。

## Acceptance Criteria

- B-001 至 B-009 在 tech spec 和 tasks 中完整映射。
- focused suite 在当前 `0.2.0` 上成功，并证明历史/缺失/空章节不能替代当前章节。
- workflow structure audit 证明 paths、5-target matrix 和 downstream needs 正确。
- implementation PR 触发 Release workflow，并产生 preflight、notes、五目标成功证据。
- Spec PR 只包含 `specs/GH271/`，implementation PR 独立创建。
