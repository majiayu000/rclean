//! AI / ML model cache rules.
//!
//! Issue #102 extension to the v0.2 "developer-grade mole" expansion.
//! Adds three rules for the largest disk consumers on modern AI dev
//! boxes — and distinguishes rebuildable model caches from user
//! data via the new [`Safety::ReportOnly`](crate::model::Safety)
//! variant.
//!
//! - `ai.huggingface_hub` (caution) — HuggingFace Hub local cache.
//!   Uses `snapshots/` softlinks pointing into `blobs/`. Direct
//!   `rm -rf` works (no cross-user hardlinks) but re-download cost
//!   can be 5 GB - 200 GB. Prefer guided cleanup via
//!   `huggingface-cli delete-cache` so the user can choose models.
//! - `ai.torch_hub` (safe) — `torch.hub.load()` weights cache.
//!   Recreated automatically on the next `torch.hub.load()` call.
//! - `ai.vllm_compile_cache` (caution) — vLLM compiled graph/kernel
//!   cache. Rebuildable, but regeneration can be slow and model/server
//!   startup-visible.
//! - `ai.whisper_models` (caution) — Whisper downloaded model cache.
//!   Re-downloads can be large and user-visible.
//! - `ai.ollama_models` (**report-only**, NOT cache) — user-pulled
//!   Ollama model weights. Equivalent to user-installed binaries:
//!   re-pulling a 70B model is hours of network time. Never selected
//!   for cleanup even with `--include-blocked`. Reported so the user
//!   can see the size and decide manually.

use std::path::Path;

use crate::model::{CandidateDraft, Category, Safety};
use crate::rules::markers::parent_ends_with;

pub fn classify(project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
    // HuggingFace hub: ~/.cache/huggingface/hub
    if name == "hub" && parent_ends_with(project_dir, &[".cache", "huggingface"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "ai.huggingface_hub".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["HuggingFace Hub model cache".to_string()],
            warnings: vec![
                "Re-downloading models from HuggingFace can take \
                 5 GB - 200 GB depending on the models in use. Prefer \
                 `huggingface-cli delete-cache` for guided per-model \
                 cleanup."
                    .to_string(),
            ],
            restore_hint: "Use `huggingface-cli delete-cache` to choose models, \
                           or the cache repopulates as `transformers`/`diffusers` \
                           re-download what is needed"
                .to_string(),
        });
    }

    // PyTorch hub: ~/.cache/torch/hub
    if name == "hub" && parent_ends_with(project_dir, &[".cache", "torch"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "ai.torch_hub".to_string(),
            category: Category::Cache,
            safety: Safety::Safe,
            reasons: vec!["PyTorch hub weights cache".to_string()],
            warnings: Vec::new(),
            restore_hint: "Recreated automatically on the next `torch.hub.load()`".to_string(),
        });
    }

    // vLLM compile cache: ~/.cache/vllm/torch_compile_cache
    if name == "torch_compile_cache" && parent_ends_with(project_dir, &[".cache", "vllm"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "ai.vllm_compile_cache".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["vLLM compiled graph/kernel cache".to_string()],
            warnings: vec![
                "Deleting vLLM compile artifacts can make the next model server startup slow while kernels and graphs are rebuilt"
                    .to_string(),
            ],
            restore_hint: "vLLM will rebuild compile artifacts on the next model/server start"
                .to_string(),
        });
    }

    // Whisper model cache: ~/.cache/whisper
    if name == "whisper" && parent_ends_with(project_dir, &[".cache"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "ai.whisper_models".to_string(),
            category: Category::Cache,
            safety: Safety::Caution,
            reasons: vec!["Whisper downloaded model cache".to_string()],
            warnings: vec![
                "Whisper model files may be large; deleting them forces re-download before the next transcription"
                    .to_string(),
            ],
            restore_hint: "Whisper will re-download the selected model on the next run".to_string(),
        });
    }

    // Ollama model store — user data, NOT cache.
    if name == "models" && parent_ends_with(project_dir, &[".ollama"]) {
        return Some(CandidateDraft {
            path: path.to_path_buf(),
            name: name.to_string(),
            rule_id: "ai.ollama_models".to_string(),
            category: Category::Deps,
            safety: Safety::ReportOnly,
            reasons: vec!["Ollama user-pulled model weights (NOT cache)".to_string()],
            warnings: vec![
                "Ollama models are user-installed weights, not a \
                 rebuildable cache. Deleting requires re-pulling each \
                 model (hours of network time for a 70B model). \
                 rclean will report the size but never auto-select \
                 this path, even with --include-blocked. Use \
                 `ollama rm <model>` for per-model removal."
                    .to_string(),
            ],
            restore_hint: "Run `ollama pull <model>` for each model you want back".to_string(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ---- HuggingFace ----

    #[test]
    fn classifies_huggingface_hub_cache() {
        let parent = PathBuf::from("/Users/me/.cache/huggingface");
        let path = parent.join("hub");
        let draft = classify(&parent, "hub", &path).expect("should classify");
        assert_eq!(draft.rule_id, "ai.huggingface_hub");
        assert_eq!(draft.safety, Safety::Caution);
        assert!(draft.restore_hint.contains("huggingface-cli"));
    }

    #[test]
    fn rejects_hub_outside_huggingface_parent() {
        let parent = PathBuf::from("/Users/me/.cache/something-else");
        let path = parent.join("hub");
        assert!(classify(&parent, "hub", &path).is_none());
    }

    // ---- PyTorch hub ----

    #[test]
    fn classifies_torch_hub_cache() {
        let parent = PathBuf::from("/Users/me/.cache/torch");
        let path = parent.join("hub");
        let draft = classify(&parent, "hub", &path).expect("should classify");
        assert_eq!(draft.rule_id, "ai.torch_hub");
        assert_eq!(draft.safety, Safety::Safe);
        assert!(draft.warnings.is_empty(), "torch hub is safe");
    }

    // ---- vLLM compile cache ----

    #[test]
    fn classifies_vllm_compile_cache() {
        let parent = PathBuf::from("/Users/me/.cache/vllm");
        let path = parent.join("torch_compile_cache");
        let draft = classify(&parent, "torch_compile_cache", &path).expect("should classify");
        assert_eq!(draft.rule_id, "ai.vllm_compile_cache");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Caution);
        assert!(
            draft.warnings.iter().any(|w| w.contains("startup slow")),
            "vLLM draft should warn about startup rebuild cost; got {:?}",
            draft.warnings
        );
    }

    #[test]
    fn rejects_torch_compile_cache_outside_vllm_parent() {
        let parent = PathBuf::from("/Users/me/.cache/torch");
        let path = parent.join("torch_compile_cache");
        assert!(classify(&parent, "torch_compile_cache", &path).is_none());
    }

    // ---- Whisper model cache ----

    #[test]
    fn classifies_whisper_models_cache() {
        let parent = PathBuf::from("/Users/me/.cache");
        let path = parent.join("whisper");
        let draft = classify(&parent, "whisper", &path).expect("should classify");
        assert_eq!(draft.rule_id, "ai.whisper_models");
        assert_eq!(draft.category, Category::Cache);
        assert_eq!(draft.safety, Safety::Caution);
        assert!(
            draft.warnings.iter().any(|w| w.contains("re-download")),
            "Whisper draft should warn about redownload cost; got {:?}",
            draft.warnings
        );
    }

    #[test]
    fn rejects_whisper_outside_xdg_cache_parent() {
        let parent = PathBuf::from("/Users/me/project");
        let path = parent.join("whisper");
        assert!(classify(&parent, "whisper", &path).is_none());
    }

    // ---- Ollama (report-only) ----

    #[test]
    fn classifies_ollama_models_as_report_only() {
        let parent = PathBuf::from("/Users/me/.ollama");
        let path = parent.join("models");
        let draft = classify(&parent, "models", &path).expect("should classify");
        assert_eq!(draft.rule_id, "ai.ollama_models");
        // The defining invariant of this PR: Ollama is ReportOnly,
        // not Caution or Blocked. Selection paths must refuse it.
        assert_eq!(draft.safety, Safety::ReportOnly);
        assert_eq!(draft.category, Category::Deps);
        assert!(
            draft.warnings.iter().any(|w| w.contains("user-installed")),
            "Ollama draft should warn about user-installed weights; got {:?}",
            draft.warnings
        );
        assert!(draft.restore_hint.contains("ollama pull"));
    }

    #[test]
    fn rejects_models_outside_ollama_parent() {
        let parent = PathBuf::from("/Users/me/project");
        let path = parent.join("models");
        assert!(classify(&parent, "models", &path).is_none());
    }

    #[test]
    fn rejects_other_names_under_ollama() {
        let parent = PathBuf::from("/Users/me/.ollama");
        let path = parent.join("logs");
        assert!(classify(&parent, "logs", &path).is_none());
    }
}
