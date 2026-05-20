# Changelog

All notable changes to `rclean` will be documented in this file.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/),
and the project adheres to semantic versioning. Pre-1.0, patch versions may
include breaking changes per semver 0.x; each break is noted explicitly.

## 0.1.3 - Unreleased (M3 candidate)

### Changed

- ActionPlan output is now `schemaVersion: 2`. Selected candidates carry a
  plan-local `id`, `category`, and `riskScore`; `deleteMode` now records
  `graveyard` when plans are written through `clean --graveyard --write-plan`.
- `schemaVersion: 1` plans are intentionally rejected with a rescan hint. Per
  the 0.x roadmap, there is no compatibility shim for old ActionPlan files.
- Graveyard manifest writes now preserve selected candidate category, safety,
  risk score, and the ActionPlan v2 candidate id when a plan-origin candidate
  is buried.

## 0.1.2 - Unreleased (M2 candidate)

Once the M2 PRs (#34, #35, #36) land, this section becomes the 0.1.2 entry.
Order below mirrors merge order.

### Added

- `Candidate.risk_score: f32` advisory signal — composite of dirty-git ×
  0.40, recent-mtime × 0.25, and missing-lockfile × 0.20. Independent of
  the safe/caution/blocked tier; surfaces as a new `Risk` column in the
  `scan` table and `riskScore` field in JSON / ActionPlan output. Current
  max is 0.85 (root-boundary axis deferred to a follow-up). [#34]
- `.rcleanignore` file at the scan root and repeatable `--ignore <GLOB>`
  CLI flag, both using `.gitignore` syntax via the `ignore` crate.
  Negation patterns (`!pattern`), deeply nested matches, and additive
  `--ignore A --ignore B` layering all work. Excluded candidates never
  enter the report, plan, or table. [#35]
- `.rclean.toml` user rules — declare new candidate names with
  `name_glob`, `parent_markers`, `category`, `safety` (`safe` or
  `caution`; `blocked` is rejected at load time). Useful for custom
  build directories that no built-in ecosystem covers. [#36]

### Compatibility

- `Candidate` gains a serde-default `risk_score` field. v0.1.0 ActionPlan
  files still load (field defaults to 0.0); new plans serialize the
  field. Stays in schemaVersion 1.

## 0.1.1 - 2026-05-19

Pure internal rebuild — no user-facing CLI behavior change.

### Changed

- `src/rules/mod.rs`'s 596-line `classify_candidate` match function split
  into 10 per-ecosystem modules (`node`, `python`, `rust`, `jvm`,
  `flutter`, `dotnet`, `ruby`, `go`, `ios`, `generic`) plus a shared
  `markers.rs`. Dispatch chain replays v0.1.0's match-arm priority
  (rust → jvm → flutter → node → … so a `build/` under a mixed Gradle
  +Node project still classifies as `java.gradle_build`, not
  `node.build`). [#28]
- `git_info` is now cached per repo root (`GitCache`). Monorepos where
  N sibling projects share one `.git` drop from `2N` git subprocess
  calls to `N+1` (one `rev-parse` per dir, one `status` per repo). [#29]
- `scan_dir` now folds each project's source size from its existing
  `stat` pass instead of a second `walkdir`. Eliminates one full
  directory walk per project. Behavior is preserved exactly. [#30]
- All public functions migrated from `Result<_, String>` to
  `thiserror`-derived enums (`ScanError`, `PlanError`, `CleanError`,
  `ParseError`, `RcleanError`). U-17 compliance. [#33]
- Six `eprintln!` sites routed through `tracing` with an `EnvFilter`
  default of `warn` (or `debug` under `--verbose`). User-facing
  confirmations (`wrote action plan: …`) and final error display
  continue to use direct stderr writes so the tracing filter doesn't
  hide them. [#33]

### Compatibility

- No CLI flag changes. No JSON / ActionPlan schema changes.
- `--verbose` semantics preserved: same messages visible at same level.

## 0.1.0 - Unreleased

Initial from-scratch Rust CLI.

### Added

- Workspace scanning for rebuildable developer artifacts.
- Human table output, JSON output, and `Biggest wins` summary.
- Safe/caution/blocked cleanup classification.
- ActionPlan write/read workflow with stale path, symlink, and root revalidation.
- Interactive numbered cleanup selection with lists, ranges, all-safe, and empty selection.
- Rules for Node, Python, Rust, Go, iOS, Java/Gradle, Flutter/Dart, .NET, Ruby, and generic coverage artifacts.
- Trash-first cleanup with explicit permanent deletion.
- CI, release packaging docs, benchmark report, and README demo asset.
