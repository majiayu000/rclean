# AI / ML model caches — spec

Status: implemented (issues #102 and #162).

This document explains the design decisions behind the AI/ML cache and
model-store rules and the `Safety::ReportOnly` variant they rely on.

## Motivation

AI/ML developers routinely accumulate **tens to hundreds of GB** in
model caches. In 2025-2026, LLM tooling makes this the new dominant
disk consumer for many machines. `rclean` had no AI/ML rules before
issue #102.

Two of these are true rebuildable caches; one is **user data, not
cache**, and the design must reflect that distinction.

## Cache and model-store rules

| Rule id | Path | Safety | Restore |
|---|---|---|---|
| `ai.huggingface_hub` | `~/.cache/huggingface/hub` (every OS) | **caution** | `huggingface-cli delete-cache` |
| `ai.torch_hub` | `~/.cache/torch/hub` (every OS) | safe | next `torch.hub.load()` |
| `ai.vllm_compile_cache` | `~/.cache/vllm/torch_compile_cache` | **caution** | next vLLM model/server start |
| `ai.whisper_models` | `~/.cache/whisper` | **caution** | next Whisper run redownloads the selected model |
| `ai.llama_cpp_cache` | `~/.cache/llama.cpp`, `~/Library/Caches/llama.cpp`, `%LOCALAPPDATA%/llama.cpp` | **report-only** | manual restore/re-download |
| `ai.ollama_models` | `~/.ollama/models` (every OS) | **report-only** | `ollama pull <model>` (per model) |
| `ai.whisper_cpp_models` | `<whisper.cpp>/models` with downloader marker | **report-only** | rerun model download script |
| `ai.comfyui_models` | `<ComfyUI>/models` with ComfyUI markers | **report-only** | restore or download models from configured sources |

## Cache vs user data: the key distinction

HuggingFace Hub and PyTorch Hub are both **caches** — content can be
re-downloaded automatically by the framework on next use. They map
to existing safety states (caution because of download cost, safe
because of automatic restore).

Ollama, llama.cpp, whisper.cpp project models, and ComfyUI models are
**not normal caches**. They contain user-pulled or user-curated model
weights — equivalent to user-installed binaries. Re-pulling a 70B
model is hours of network time the user explicitly chose to spend.

If `rclean` treated Ollama like a normal "cache" rule with
`Safety::Caution`, an unguarded `clean --all --include-caution`
would destroy days of bandwidth. If it used `Safety::Blocked`, the
user could still opt in via `--include-blocked` — also wrong, because
the user-data semantic isn't about "we're uncertain whether this is
safe", it's about "this isn't recoverable from upstream like a cache
is."

## The new `Safety::ReportOnly` variant

Issue #102 added a fifth Safety state:

```rust
pub enum Safety {
    Safe,
    Caution,
    Blocked,
    ReportOnly,  // user data, surface for awareness, never select
    Unknown,
}
```

### Semantics

- **Always reported** in `scan` output and JSON — the user sees the
  size and the rule id so they can decide manually whether to
  remove via the upstream tool.
- **Never selected** by `clean --all`, even with
  `--include-caution` and `--include-blocked` together.
- **Never offered** in interactive selection (`clean` without
  `--all` and the TUI selector skip the row, same as Blocked).
- **Plan replay refuses** to act on a ReportOnly candidate, even if
  a user hand-edits an ActionPlan JSON to include one.
- **Exit code** for `explain <path>` matches Blocked (exit 4) — both
  states refuse cleanup, so callers don't need to distinguish unless
  they want to.

### Why a new variant instead of reusing Blocked

| Variant | Meaning |
|---|---|
| `Blocked` | "rclean is uncertain or this is in a protected zone; let the user override with `--include-blocked` if they know what they're doing" |
| `ReportOnly` | "this is user data, not a cache; the upstream tool owns the lifecycle. `--include-blocked` will not change this." |

The semantics are different enough that conflating them would either
weaken Blocked (by allowing destructive overrides on Ollama) or
weaken the user-data guard (by training users that
`--include-blocked` always lets them through).

## TUI / output presentation

- TUI glyph: `[#]` (distinct from Blocked's `[×]`)
- TUI color: magenta (distinct from Blocked's dark gray)
- Table output: shows `report-only` in the safety column
- JSON: `"safety": "report-only"` (serde rename so the user-facing
  string is kebab-case while the Rust enum variant stays CamelCase)

## Out of scope (Tier 3 / single-source)

- TGI custom model/cache overrides — they are arbitrary user paths.
  Default TGI/HuggingFace downloads remain covered by
  `ai.huggingface_hub`.
- `sentence-transformers` (uses HuggingFace under the hood, already
  covered)
- `/usr/share/ollama/.ollama/models` (system-wide Ollama install
  path) — out of `--home` scope; users can `scan` the explicit path
  if needed, but `rclean` will still classify it ReportOnly
- Arbitrary `models`, `downloads`, or `llama.cpp` directories outside
  the exact anchors and markers listed above.

## Anti-patterns this design prevents

- A user runs `clean --all --include-caution --include-blocked` to
  reclaim everything possible and loses 200 GB of Ollama model
  weights they spent the weekend pulling on a hotel wifi.
- A hand-edited ActionPlan that lists `~/.ollama/models` as a
  selected candidate gets replayed and silently destroys data —
  `plan.rs` refuses such replay with an explicit error.
