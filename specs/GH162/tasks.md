# GH162 tasks: AI and ML model store coverage

- [x] Define accepted and rejected AI/ML model-store paths.
- [x] Add `ai.llama_cpp_cache` as a report-only exact global cache anchor.
- [x] Add `ai.whisper_cpp_models` with a downloader-script marker.
- [x] Add `ai.comfyui_models` with strong ComfyUI project markers.
- [x] Update catalog, doctor, home scan roots, README, and AI model docs.
- [x] Add positive tests for each new rule id.
- [x] Add negative tests for arbitrary model/download/cache names without
  markers.
- [x] Add selection test proving report-only llama.cpp candidates are not
  selected by clean.
- [ ] Land after fresh CI and maintainer review.
