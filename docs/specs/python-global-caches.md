# Python global caches — spec

Status: implemented (issue #101).

This document explains the design decisions behind the three Python
global-cache rules introduced in issue [#101](https://github.com/majiayu000/rclean/issues/100):

- `python.uv_cache` — caution
- `python.poetry_cache` — safe
- `python.pipx_cache` — safe

## Motivation

`rclean` already covers project-local Python artifacts (`.venv`,
`__pycache__`, `.pytest_cache`, …) and the legacy `pip` global cache.
Modern Python tooling adds three more globally-cached package managers
that routinely accumulate multi-GB of reclaimable space:

- **uv** (Astral) — fastest-growing Python package manager; ships a
  hardlink/reflink cache.
- **Poetry** — long-standing project workflow tool; isolated wheels
  cache.
- **pipx** — global CLI installer; `pipx run` ephemeral venvs
  accumulate quickly.

On the empirical dev box this rule was authored against,
`~/.cache/uv` alone was **2.5 GB** with no `rclean` rule covering it.

## Why uv needs a dual path on macOS

Per the [`dirs` crate convention](https://docs.rs/dirs/latest/dirs/fn.cache_dir.html),
uv's macOS default is `~/Library/Caches/uv`. In practice, when
`XDG_CACHE_HOME` is set, uv config opts in, or the user follows the
XDG conventions explicitly, uv resolves to `~/.cache/uv` instead.

Cross-validation during issue authoring surfaced this disagreement:

- Gemini reported `~/.cache/uv` as the macOS default.
- Grok corrected to `~/Library/Caches/uv` as the platform-native
  default with `~/.cache/uv` as the XDG fallback.
- The local empirical machine resolved to `~/.cache/uv` (2.5 GB).

A rule that only checked one path would miss real user data. The
classifier therefore accepts **both** anchors on every OS. `doctor`
reports `python.uv_cache` as applicable when either anchor exists.

The same XDG-on-macOS pattern is honoured by Poetry and pipx, so all
three rules share `is_user_cache_parent` to avoid drift.

## Why uv is caution

uv builds a content-addressable cache with hardlinks/reflinks into
project `.venv` directories. Direct `rm -rf` may leave dangling
links in active venvs that the user has not yet rebuilt. The safe
restore path is `uv cache clean`, which uv ships specifically for
this scenario.

Poetry and pipx do not use hardlinks into project venvs (Poetry
copies wheels into per-venv installs, pipx writes self-contained
venvs for each application), so both are classified `safe`.

## --home expansion

On macOS the `--home` expansion now adds `~/.cache` to the candidate
roots, alongside `~/Library/Caches`. Without this addition, a user
with an XDG override would have `~/.cache/uv` on disk but `rclean
scan --home` would never walk into it. Adding `~/.cache` to the
macOS branch is strictly additive — if it does not exist the walker
skips it.

## doctor anchor selection

Each rule's `doctor` entry uses `check_any_anchor` with the same
canonical list as the classifier (native + XDG on macOS, XDG only
on Linux, `%LOCALAPPDATA%\<tool>\Cache` on Windows). The first
existing path is reported as the applicable anchor.

## Out of scope (deferred)

- `pdm`, `hatch`, `conda`, `mamba` — path / hardlink complexity;
  `conda` especially needs `conda clean`, not `rm -rf`, to avoid
  breaking environments.
- `ruff` and `mypy` caches — already project-local and covered by
  the existing `python.ruff` / `python.mypy` rules.
