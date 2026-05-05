# Changelog

All notable changes to `rclean` will be documented in this file.

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
