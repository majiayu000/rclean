# Release Tag Version Preflight - Product Spec

## Linked Issue

- GitHub issue: `#268`
- URL: `https://github.com/majiayu000/rclean/issues/268`
- Locale: `zh-CN`
- Route: `write_spec`
- Complexity: `medium`

## Summary

在 tag 发布进入多平台构建前，强制校验 tag 必须精确等于
`v<rclean-cli Cargo package version>`。版本缺失、格式错误或不一致必须显式失败；PR
上的 release workflow dry-run 继续可用，并用同一校验器覆盖成功与失败路径。

## Problem

当前 `.github/workflows/release.yml` 直接从 `GITHUB_REF_NAME#v` 得到归档、changelog、
GitHub Release 和 Homebrew formula 的版本，而二进制版本来自 `Cargo.toml`。二者没有
自动相等性检查。当前主线 package/tag 都是 `0.2.0`，但模拟 tag `v9.9.9` 时，现有
表达式会生成 `rclean-cli-v9.9.9-...` 归档名，其中仍是 `rclean 0.2.0` 二进制。

`docs/release/release-checklist.md` 和既有 #188/#213 只要求人工确认，无法在误打 tag 时
fail closed。错误会继续传播到 draft release 和 Homebrew tap，形成用户可见的版本错配。

## Goals

- tag 发布只接受精确的 `v<current Cargo package version>`。
- 缺少 `v`、空 tag、错误版本和 package/tag 不一致都输出明确错误并非零退出。
- 校验在多平台 release build 前运行，失败时 build、draft release、Homebrew bump 均不执行。
- PR release dry-run 不把 PR ref 当成 release tag，并继续验证构建矩阵。
- 正负契约使用同一仓库校验器，避免 workflow 内复制解析逻辑。
- 保持人工 draft-release 发布与 `cargo publish` gate 不变。

## Non-Goals

- 不自动创建、删除、移动或重新打 Git tag。
- 不自动发布 GitHub Release 或 crates.io package。
- 不改变 semver policy、version bump 流程、changelog 格式或 Homebrew formula 内容。
- 不修改 Rust runtime、CLI、scan、clean、ActionPlan、删除或路径安全行为。
- 不在实现中顺带改变 `TAP_PUSH_TOKEN` 缺失策略或 GitHub Action 版本。

## Behavior Invariants

1. **B-001** tag `vX.Y.Z` 仅当 `X.Y.Z` 精确等于 `cargo metadata` 中 package
   `rclean-cli` 的 version 时通过。
2. **B-002** 空输入、无 `v` 前缀、仅 `v`、格式错误或版本不匹配均明确报错并非零退出。
3. **B-003** release workflow 在 tag 事件中使用仓库内唯一校验器，不在 YAML 中复制第二套
   Cargo version 解析/比较逻辑。
4. **B-004** 多平台 `build` job 显式依赖 preflight；现有 `release -> build`、
   `bump-tap -> release` 链保证失败后没有归档、draft release 或 tap 外部更新。
5. **B-005** pull request 事件不校验 PR ref 与 Cargo version，相反运行同一校验器的
   exact-match、missing-prefix、mismatch 确定性契约，并保留现有 release build dry-run。
6. **B-006** 校验器在找不到 `rclean-cli` package、无法读取 metadata 或依赖工具失败时
   fail closed，不返回猜测版本或 warning fallback。
7. **B-007** 人工 draft publish、tag 创建、crates.io publish 和 Homebrew token gate 保持不变。
8. **B-008** implementation scope 仅允许 `.github/workflows/release.yml`、一个仓库校验脚本
   及其确定性测试脚本；stable/MSRV/VibeGuard/current-head CI 全部通过。

## Acceptance Criteria

- B-001 至 B-008 在 tech spec 和 tasks 中完整映射。
- focused contract tests 至少覆盖 exact match、missing `v` 和 mismatched version。
- workflow dependency audit 证明 build 依赖 preflight，release/tap 仍沿既有依赖链串联。
- PR workflow run 证明 release-notes dry-run 与五目标 build matrix 仍可完成。
- Spec PR 只包含 `specs/GH268/`，implementation PR 独立创建。
