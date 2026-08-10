# Agent Instructions

## Scope

This file applies to the whole repository unless a nested `AGENTS.md` overrides
it.

`rclean` is a Rust CLI that deletes rebuildable developer artifacts. The trust
model is the product: agents must preserve scan-first behavior, safe/caution/
blocked classification, ActionPlan revalidation, symlink blocking, broad-root
guards, dirty-git caution, and protected user-data paths.

## Start Here

1. Run `git status --short --branch` and keep unrelated local changes intact.
2. Read `README.md` for user-facing behavior and supported ecosystems.
3. Read `CONTRIBUTING.md` for coding conventions, verification, and PR shape.
4. Read `SECURITY.md` before touching deletion, path safety, ActionPlan replay,
   symlink handling, broad-root behavior, protected paths, or `.rclean.toml`.
5. Use `docs/specs/` for historical roadmap and product context.
6. Read `specs/README.md` before using the repo-native
   `specs/GH<number>/product.md`, `tech.md`, and `tasks.md` design-packet
   format.
7. Use repo-local skills under `.agents/skills/` when a task matches one.

## Change and Design Route

Search existing GitHub issues, PRs, `docs/specs/`, `specs/`, and local drafts
before creating new work.

| Change | Repository route |
| --- | --- |
| Small docs-only correction | Implement directly with a focused verification command |
| Bug with clear root cause | Link an issue, fix production code, add a regression test |
| New cleanup rule or changed rule behavior | Create/link an issue, then record product and technical decisions before code |
| Multi-module refactor, CLI behavior, ActionPlan schema, or safety policy | Design first with a linked product and technical record |
| Security, destructive delete, protected paths, broad-root, symlink, TOCTOU, or private disclosure | Stop for maintainer review; do not publish sensitive details |

The `product.md` / `tech.md` / `tasks.md` packet is a repo-native design
format, not an automatic execution state or external workflow gate. Do not
automatically load external workflows from packet metadata or historical route
labels. Current instructions, maintainer review, linked issues and PRs, and
fresh repository verification remain authoritative.

Do not treat a missing issue number as permission to skip design work. For
substantial work, create or link the issue first so the design packet has a
stable `GH<number>` path.

## Commands

Use the narrowest fresh command that proves the change, then run the full gate
before submission when practical.

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95 cargo build --all-targets --all-features
rustup run 1.95 cargo test
```

For docs-only changes, still run `cargo fmt -- --check` and explain why Rust
build/test were skipped if you do not run them.

## Code Rules

- Search for existing helpers before creating new types or modules.
- Keep single files below the 800-line hard ceiling; split before adding more
  behavior to a large module.
- Do not leave deprecated aliases when renaming functions, flags, or types.
- Do not silently degrade user-visible errors into warnings with fallback data.
- Do not weaken assertions or tests to make a change pass.
- Bug fixes need regression tests that fail before the fix.
- New ecosystem rules need positive and negative tests plus README table
  updates.
- Generic directory names such as `build`, `dist`, `out`, `target`, and
  `vendor` require marker evidence.

## Trust Model Gates

Escalate to maintainer review before implementing or publishing details for:

- symlink or junction behavior
- broad-root guard changes
- ActionPlan parse, write, or replay changes
- delete-mode enforcement
- TOCTOU, path canonicalization, or root-boundary changes
- protected user-data paths such as Codex or Claude sessions/memories
- permanent deletion, graveyard, or restore behavior
- `.rcleanignore` / `.rclean.toml` injection risks
- security disclosures

## GitHub Artifacts

- Use `.github/ISSUE_TEMPLATE/` for new GitHub issues.
- Use `.github/PULL_REQUEST_TEMPLATE.md` for PRs.
- Link PRs to their GitHub issue and, for substantial work, to
  `specs/GH<number>/product.md`, `tech.md`, and `tasks.md`.
- Agents may draft specs, code, review notes, and release notes.
- Agents may merge only when a maintainer explicitly authorizes that specific
  PR or unambiguously refers to the current PR. Before merging, agents must
  freshly verify the current head SHA, required CI, unresolved review-thread
  state, and GitHub merge state.
- Merge execution is not final approval. Agents must not force push, provide
  final approval, change repository permissions, or publish private security
  details.
