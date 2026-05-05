# GitHub Trending And Competitor Analysis

Date: 2026-05-06

Note: the requested `githubtrending` skill is not available in this Codex
environment's advertised skill list. A local `github-trending` skill file was
found and used as the analysis framework, with live web/GitHub-adjacent search
results as data sources.

## Findings

### Multi-Ecosystem Cleanup Is Already A Known Category

Relevant projects and references:

- `kondo` cleans `node_modules`, `target`, `build`, and related artifacts across
  project types.
- `clean-dev-dirs` is a Rust CLI for recursively cleaning Rust `target`, Node
  `node_modules`, Python caches, and Go `vendor`.
- `dev-sweep` and similarly named packages already position around the same
  local developer bloat problem.

Implication for `rclean`: raw rule coverage is not enough. The product must win
on trust, reviewability, and launch packaging.

### Screenshot-Friendly Output Matters

`sweep-cli` highlights full project size, artifact size, and junk percentage.
This is useful because it produces a strong first-run story: "this project is
97% rebuildable artifacts."

Implication for `rclean`: the next output optimization should add project total
size and artifact percentage, then show "Biggest wins" near the top.

### Interactive Safety Is A Trend

Recent Rust CLI/TUI projects get attention when they make risky operations easy
to inspect before acting. Several branch-cleaning tools emphasize TUI/selection
workflows with safety checks.

Implication for `rclean`: keep the CLI path scriptable, but consider a Ratatui
TUI only after ActionPlan and safety policy are stable. The current numbered
selection is a good intermediate step.

### Packaging Is Part Of The Product

Rust CLIs that spread well usually provide:

- clear package/binary naming
- `cargo install`
- GitHub Release binaries
- Homebrew formula
- first-screen README demo
- copy-paste examples

Implication for `rclean`: `rclean-cli` package plus `rclean` binary is the right
path because `rclean` is already occupied on crates.io.

## Optimization Decisions Applied

- Added package metadata and MIT license.
- Strengthened README positioning around trust and ActionPlan review.
- Added "Biggest wins" above the full scan table.
- Added per-project total bytes and artifact percentage to reports.
- Added a README demo asset for first-run sharing.
- Added an initial changelog for release packaging.
- Kept `rclean-cli` as package name and `rclean` as binary name.
- Documented release and packaging flow.

## Release-Blocked Work

These should wait until the repository is public and the release URL is known:

1. Record a terminal GIF from the real 445 GB benchmark.
2. Add a real Homebrew formula with final GitHub Release URLs and checksums.
3. Consider Ratatui TUI after the non-TUI workflow is complete.

## Sources

- https://docs.rs/crate/kondo/latest
- https://lib.rs/crates/clean-dev-dirs
- https://pypi.org/project/sweep-cli/
- https://pypi.org/project/devbroom/
- https://docs.rs/crate/dev-sweep/0.1.3
