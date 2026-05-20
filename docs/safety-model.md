# Safety model

`rclean`'s product **is** its safety model. Every other feature
(ActionPlan, `explain`, the rule catalog, the broad-root guard) is a
mechanism for making that model auditable, scriptable, and hard to
weaken by accident. This document is the canonical reference for
the four safety states and the demotion rules that produce them.

## The four states

| State | Meaning | Selectable by `clean --all`? | Visible in report? |
|---|---|---|---|
| `safe` | A built-in or user rule classified the path and no safety check fired. | **Yes** (default). | Yes. |
| `caution` | Classified, but at least one demotion fired (dirty git, project-marker concern, etc.). | Only with `--include-caution`. | Yes. |
| `blocked` | Classified, but a safety check disqualified the path entirely (symlink, runtime/system path, scan-root violation). | **Never** — `--include-blocked` only affects visibility, not selectability. | Only with `--include-blocked`. |
| `unknown` | No rule matched the path. | Never. | Only via `explain` (never inside a scan report). |

A path is in exactly one state at a time. There is no
"caution + blocked" combined state — the most restrictive applicable
rule wins.

## Why four states (not two)

A single safe/unsafe split fails real workflows in two ways:

1. **`caution` is the "ask the human" tier.** A dirty git tree
   doesn't mean a `target/` directory is dangerous to clean — it
   means the project owner might be in the middle of something.
   Demoting to `caution` and gating selection behind
   `--include-caution` lets bulk cleans skip those projects while
   leaving the candidate visible for interactive review.
2. **`blocked` is the "refuse on principle" tier.** A symlinked
   `node_modules` could resolve to anywhere on disk. No flag
   re-enables selection — the only path is to remove the symlink
   first. This is intentionally one-way.

`unknown` exists for `explain` only: the report would not list a
path no rule matched, so `unknown` is the exit-code signal for
"this isn't in scope" rather than a Safety variant the report ever
needs to render.

## Demotion rules

A candidate starts wherever its rule places it (typically `safe`).
A chain of demotions can move it down toward `blocked` but never up
toward `safe`.

| Demotion | Trigger | New state | Implemented in |
|---|---|---|---|
| Symlink | `fs::symlink_metadata(path).file_type().is_symlink()` is true at scan time. | `blocked` | `src/scan.rs::apply_path_safety` |
| Runtime/system path | Any component of the candidate path is in the protected allowlist (`.cargo`, `.rustup`, `.nvm`, `.fnm`, `.pyenv`, `.sdkman`, `.rbenv`, `.conda`, `Library`, `Applications`, `.Trash`). | `blocked` | `src/scan.rs::is_runtime_or_system_path` |
| Outside scan root | `candidate.canonicalize()` does not start with `root.canonicalize()`. | `blocked` | `src/scan.rs::apply_path_safety` |
| Dirty git worktree | `git status --porcelain` is non-empty for the enclosing repo. Only demotes `safe → caution`; never promotes/demotes anything else. | `caution` | `src/scan/project.rs::build_project_report` (via `GitCache`) |
| Generic-name without marker | `build` / `dist` / `out` / `target` / `vendor` and no ecosystem marker in the parent. | Not classified at all (no draft produced). | `src/rules/generic.rs`, `src/rules/markers.rs` |
| Python `venv` without venv marker | Bare `venv` / `.venv` directory whose contents don't look like a virtualenv. | Not classified. | `src/rules/python.rs` |

Two non-obvious cases that the rules deliberately do *not* demote:

- A `safe` candidate inside a *non-existent* git repo (no `git
  rev-parse` success) stays `safe`. Absence of git ≠ dirty.
- A `safe` candidate whose parent has uncommitted changes but the
  candidate itself is in `.gitignore` (which it normally is) still
  demotes to `caution`. Demotion is per-project, not per-candidate
  path.

## Selectability matrix

Selectability is what `clean --all` actually picks, not what the
table renders.

| | `safe` | `caution` | `blocked` | `unknown` |
|---|---|---|---|---|
| `clean --all` default | ✅ select | ❌ skip | ❌ skip | n/a |
| `clean --all --include-caution` | ✅ select | ✅ select | ❌ skip | n/a |
| `clean --all --include-blocked` | ✅ select | ❌ skip | ❌ **still skip** | n/a |
| Interactive (no `--all`) | numbered | numbered | not listed | n/a |
| `clean --plan` replay | re-classified | re-classified | rejected | rejected |

The single most important row: **`--include-blocked` does not make
`blocked` selectable.** It only changes report visibility. There is
no `--allow-blocked` flag, by design.

## The `clean --plan` revalidation extension

`clean --plan` re-runs every classifier check at delete time
([details](action-plan-format.md#replay-semantics)). A path that
was `safe` at scan time and is `blocked` now (because someone
symlinked it in between) is rejected. The plan's `safety` field is
informational; the live re-classification is what gates deletion.

This means: even a hand-edited plan that upgrades a `blocked` path
to `safe` will be re-classified and rejected. The trust boundary
is the *code*, not the *file on disk*.

## The broad-root guard

Independent of safety state: `clean` refuses to operate inside `/`,
`$HOME`, `/etc`, `/usr` (and the equivalent Windows / Linux paths)
unless `--allow-broad-root` is passed. This is a second-layer
guard, not a Safety variant — it fires *after* selection but
*before* deletion.

## What the safety model does *not* try to do

- It does not enforce free-disk-space minimums.
- It does not look at file ownership / permissions to decide safety.
- It does not detect "this project still has uncommitted output the
  user cares about" beyond the dirty-git heuristic.
- It does not protect against a user with write access to their own
  scan tree who chooses `clean --all --permanent`. The point is to
  make the choice deliberate, not impossible.

If you need any of those, build them as a wrapper on top of
`explain` — its exit-code contract is the right integration surface.

## Risk score (advisory, not gating)

A composite `risk_score ∈ [0.0, 0.85]` ships alongside every
candidate. It is **never** consulted by `clean`. It exists for TUI
coloring, agent plan ranking, and similar advisory consumers.

| Axis | Weight | Tripped when |
|---|---:|---|
| dirty git worktree | 0.40 | The enclosing repo has uncommitted changes. |
| recent activity | 0.25 | Project newest mtime within 7 days. |
| no lockfile | 0.20 | No `Cargo.lock` / `package-lock.json` / equivalent in the project. |
| root-boundary | 0.15 | Deferred — currently always `0.0`. |

The weight slot for `root-boundary` is reserved so that downstream
thresholds don't need to shift when it lands later. See
[`docs/specs/v0.1.x-roadmap.md`](specs/v0.1.x-roadmap.md) §4.6.

`risk_score` is independent of `safety` on purpose: a `safe`
candidate can still have a high risk score (recent + no lockfile +
dirty), and a `blocked` candidate still gets a risk score reported.
Do not fold them together — they answer different questions:

- `safety` answers "can I delete this?"
- `risk_score` answers "how cautiously should I present this to a
  human reviewer?"

## Related

- [`docs/explain-mode.md`](explain-mode.md) — single-path
  inspector, exit-code contract that mirrors the safety states.
- [`docs/action-plan-format.md`](action-plan-format.md) — replay
  contract and the four-phase revalidation it runs.
- [`docs/architecture.md`](architecture.md) — where each demotion
  rule lives in the code.
- [`SECURITY.md`](../SECURITY.md) — threat model and disclosure
  workflow.
