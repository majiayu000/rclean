# GH162 product spec: AI and ML model store coverage

## Summary

Expand AI/ML disk reporting beyond HuggingFace, PyTorch, vLLM, Whisper, and
Ollama while preserving the user-data boundary. Newly recognized model stores
are reported for visibility but never selected for cleanup.

## Problem

AI developer workflows often place large model weights in tool-specific
directories. Some of those directories are rebuildable caches, but many are
user-curated model stores. Treating every `models` or `downloads` directory as
cleanable would risk deleting expensive or manually chosen assets.

## Goals

- Report exact llama.cpp global cache roots.
- Report `whisper.cpp/models` only when the downloader script marker is
  present.
- Report `ComfyUI/models` only when strong ComfyUI project markers are
  present.
- Keep all new model-store rules `report-only`.
- Document why TGI custom cache paths and arbitrary model directories are out
  of scope.

## Non-goals

- No classification of arbitrary `models`, `downloads`, or custom cache paths.
- No direct integration with TGI cache overrides such as custom HuggingFace
  cache locations.
- No automatic deletion of model weights, checkpoints, LoRAs, embeddings, or
  other user-curated assets.
- No cleanup of system-wide model stores outside user-selected scan roots.

## Safety Policy

The new rules are `report-only` because the selected files are model weights or
model-store roots. They may be redownloadable, but redownload cost and user
intent are not reliably recoverable from the path alone.

## Done When

- Specs describe accepted and rejected paths.
- Rule code uses exact anchors or strong markers.
- Positive and negative tests cover all new rule ids.
- `scan --home` reaches the llama.cpp global cache.
- `clean --all --include-caution --include-blocked` does not select the new
  report-only candidates.
- README, `rclean rules`, doctor output, and AI model cache docs are updated.
