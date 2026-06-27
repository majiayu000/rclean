# GH162 technical spec: AI and ML model store coverage

## Rule IDs

| Rule id | Match | Safety | Category |
| --- | --- | --- | --- |
| `ai.llama_cpp_cache` | `~/.cache/llama.cpp`, `~/Library/Caches/llama.cpp`, or `%LOCALAPPDATA%/llama.cpp` | report-only | deps |
| `ai.whisper_cpp_models` | `<whisper.cpp>/models` with `models/download-ggml-model.sh` present | report-only | deps |
| `ai.comfyui_models` | `<ComfyUI>/models` with `folder_paths.py` and either `main.py` or `extra_model_paths.yaml.example` present | report-only | deps |

## Out-of-scope Candidates

- TGI default model downloads remain covered through HuggingFace cache rules.
  TGI custom override paths are arbitrary and must not be guessed.
- Bare `models`, `downloads`, `llama.cpp`, `ComfyUI`, or `whisper.cpp`
  directories outside the accepted anchors/markers must not classify.
- Existing `ai.vllm_compile_cache` and `ai.whisper_models` remain `caution`
  because they are narrow rebuildable caches; this PR does not change those
  semantics.

## Files

- Rule code: `src/rules/ai_models.rs`
- Candidate prefilter and global-rule classification: `src/rules/project.rs`
- Rule catalog: `src/rules/catalog.rs`
- Home scan roots: `src/cli.rs`
- Doctor output: `src/doctor.rs`
- Tests: `tests/rules.rs`, `tests/cli.rs`
- Docs: `README.md`, `docs/specs/ai-model-caches.md`

## Verification

Focused verification:

```sh
cargo fmt -- --check
cargo test --test rules ai_
cargo test --test cli home_flag_reports_llama_cpp_cache_as_report_only_never_selected
cargo test --test cli rules_lists_every_classifier_emitted_id
cargo test --test cli doctor_prints_rule_status_table
cargo check
```

Full gate before merge:

```sh
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95 cargo build --all-targets --all-features
rustup run 1.95 cargo test
```
