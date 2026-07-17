use std::path::Path;

use super::anchors::{
    android_sdk_anchors, browser_cache_anchors, deno_cache_anchors, homebrew_download_anchors,
    ide_log_anchors, ide_system_anchors, python_cache_anchors, simple_cache_anchors,
};
use super::{DoctorEntry, check_anchor, check_any_anchor};

pub(super) fn collect(home: &Path) -> Vec<DoctorEntry> {
    let mut entries = vec![
        check_anchor(
            "cargo.registry_cache",
            home.join(".cargo").join("registry"),
            "no Cargo install detected",
        ),
        check_anchor(
            "cargo.git_db",
            home.join(".cargo").join("git"),
            "no Cargo git deps detected",
        ),
        check_any_anchor(
            "homebrew.downloads",
            homebrew_download_anchors(home),
            "no Homebrew download cache detected",
        ),
        check_any_anchor(
            "android_sdk.download_intermediates",
            android_sdk_anchors(home),
            "no Android SDK root detected",
        ),
        check_anchor(
            "android_sdk.legacy_build_cache",
            home.join(".android").join("build-cache"),
            "no legacy Android build cache detected",
        ),
        check_anchor(
            "dart.pub_hosted_cache",
            home.join(".pub-cache").join("hosted"),
            "no Dart pub hosted cache detected",
        ),
        check_anchor(
            "dart.pub_git_cache",
            home.join(".pub-cache").join("git"),
            "no Dart pub git cache detected",
        ),
        check_anchor(
            "go.module_download_cache",
            home.join("go").join("pkg").join("mod").join("cache"),
            "no Go module cache detected",
        ),
        check_anchor(
            "go.module_cache",
            home.join("go").join("pkg").join("mod"),
            "no Go module cache detected",
        ),
        check_anchor(
            "gradle.caches",
            home.join(".gradle"),
            "no Gradle install detected",
        ),
        check_anchor(
            "maven.local_repo",
            home.join(".m2"),
            "no Maven install detected",
        ),
        check_anchor(
            "node.npm_cacache",
            home.join(".npm"),
            "no npm install detected",
        ),
        check_anchor(
            "node.npm_transient",
            home.join(".npm"),
            "no npm install detected",
        ),
        check_anchor(
            "bun.cache",
            home.join(".bun").join("install"),
            "no bun install cache detected",
        ),
        check_anchor(
            "pre_commit.cache",
            home.join(".cache"),
            "no XDG cache directory",
        ),
        check_anchor(
            "ruby.bundle_compact_index",
            home.join(".bundle").join("cache").join("compact_index"),
            "no Bundler compact index detected",
        ),
        check_anchor(
            "cloud.kube_cache",
            home.join(".kube").join("cache"),
            "no Kubernetes cache detected",
        ),
        check_anchor(
            "cloud.gcloud_logs",
            home.join(".config").join("gcloud").join("logs"),
            "no gcloud logs detected",
        ),
        check_anchor(
            "editor.vscode_obsolete_extension",
            home.join(".vscode").join("extensions"),
            "no VS Code extensions detected",
        ),
        check_anchor(
            "editor.cursor_obsolete_extension",
            home.join(".cursor").join("extensions"),
            "no Cursor extensions detected",
        ),
        check_anchor(
            "claude.old_version",
            home.join(".local")
                .join("share")
                .join("claude")
                .join("versions"),
            "no Claude Code versions detected",
        ),
    ];

    let mut pnpm_anchors = vec![home.join(".pnpm-store")];
    #[cfg(target_os = "macos")]
    {
        pnpm_anchors.push(home.join("Library").join("pnpm").join("store"));
        pnpm_anchors.push(
            home.join("Library")
                .join("Caches")
                .join("pnpm")
                .join("store"),
        );
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        pnpm_anchors.push(home.join(".local").join("share").join("pnpm").join("store"));
    }
    #[cfg(target_os = "windows")]
    {
        pnpm_anchors.push(
            home.join("AppData")
                .join("Local")
                .join("pnpm")
                .join("store"),
        );
    }
    entries.push(check_any_anchor(
        "node.pnpm_store",
        pnpm_anchors,
        "no pnpm store detected",
    ));

    // pip uses different anchors per platform.
    #[cfg(target_os = "macos")]
    {
        entries.push(check_anchor(
            "pip.cache",
            home.join("Library").join("Caches"),
            "no Library/Caches directory",
        ));
        entries.push(check_anchor(
            "go.build_cache",
            home.join("Library").join("Caches").join("go-build"),
            "no Go build cache detected",
        ));
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        entries.push(check_anchor(
            "pip.cache",
            home.join(".cache"),
            "no XDG cache directory",
        ));
        entries.push(check_anchor(
            "go.build_cache",
            home.join(".cache").join("go-build"),
            "no Go build cache detected",
        ));
    }
    #[cfg(target_os = "windows")]
    {
        entries.push(check_anchor(
            "pip.cache",
            home.join(".cache"),
            "no XDG cache directory",
        ));
        entries.push(check_anchor(
            "go.build_cache",
            home.join("AppData").join("Local").join("go-build"),
            "no Go build cache detected",
        ));
    }

    // AI / ML model caches (#102). All three rules anchor under
    // `~/.cache/...` and `~/.ollama/...` on every platform.
    entries.push(check_anchor(
        "ai.huggingface_hub",
        home.join(".cache").join("huggingface").join("hub"),
        "no HuggingFace cache detected",
    ));
    entries.push(check_anchor(
        "ai.torch_hub",
        home.join(".cache").join("torch").join("hub"),
        "no PyTorch hub cache detected",
    ));
    entries.push(check_anchor(
        "ai.vllm_compile_cache",
        home.join(".cache").join("vllm").join("torch_compile_cache"),
        "no vLLM compile cache detected",
    ));
    entries.push(check_anchor(
        "ai.whisper_models",
        home.join(".cache").join("whisper"),
        "no Whisper model cache detected",
    ));
    entries.push(check_any_anchor(
        "ai.llama_cpp_cache",
        simple_cache_anchors(home, "llama.cpp"),
        "no llama.cpp model cache detected",
    ));
    entries.push(check_anchor(
        "ai.ollama_models",
        home.join(".ollama").join("models"),
        "no Ollama install detected",
    ));

    // Python global tooling caches (#101). uv, Poetry, and pipx each
    // resolve to either the native macOS path or the XDG override —
    // real users hit both, so doctor accepts either anchor.
    entries.push(check_any_anchor(
        "python.uv_cache",
        python_cache_anchors(home, "uv"),
        "no uv install detected",
    ));
    entries.push(check_any_anchor(
        "python.poetry_cache",
        python_cache_anchors(home, "pypoetry"),
        "no Poetry install detected",
    ));
    entries.push(check_any_anchor(
        "python.pipx_cache",
        python_cache_anchors(home, "pipx"),
        "no pipx install detected",
    ));

    // Deno's cache can be native macOS or XDG-style, depending on
    // platform and user environment.
    entries.push(check_any_anchor(
        "js.deno_cache",
        deno_cache_anchors(home),
        "no Deno install detected",
    ));

    // Puppeteer keeps Chrome for Testing downloads in a global cache.
    entries.push(check_any_anchor(
        "browser.puppeteer",
        browser_cache_anchors(home, "puppeteer"),
        "no Puppeteer install detected",
    ));
    entries.push(check_any_anchor(
        "jetbrains.system_caches",
        ide_system_anchors(home, "JetBrains"),
        "no JetBrains IDE system cache detected",
    ));
    entries.push(check_any_anchor(
        "jetbrains.logs",
        ide_log_anchors(home, "JetBrains"),
        "no JetBrains IDE logs detected",
    ));
    entries.push(check_any_anchor(
        "android_studio.system_caches",
        ide_system_anchors(home, "Google"),
        "no Android Studio system cache detected",
    ));
    entries.push(check_any_anchor(
        "android_studio.logs",
        ide_log_anchors(home, "Google"),
        "no Android Studio logs detected",
    ));
    entries
}
