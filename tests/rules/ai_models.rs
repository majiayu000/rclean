use super::common::{make_dir, make_non_empty_path};
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn vllm_compile_cache_and_whisper_models_are_classified_under_xdg_cache() {
    let temp = TempDir::new().unwrap();
    let xdg = temp.path().join(".cache");
    make_non_empty_path(&xdg.join("vllm").join("torch_compile_cache"));
    make_non_empty_path(&xdg.join("whisper"));

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args(["scan", xdg.to_str().unwrap(), "--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"ruleId\": \"ai.vllm_compile_cache\"",
        ))
        .stdout(predicate::str::contains(
            "\"ruleId\": \"ai.whisper_models\"",
        ))
        .stdout(predicate::str::contains("\"safety\": \"caution\""));
}

#[test]
fn llama_cpp_cache_is_report_only_under_exact_cache_anchors() {
    let temp = TempDir::new().unwrap();
    let xdg = temp.path().join(".cache");
    let library_caches = temp.path().join("Library").join("Caches");
    make_non_empty_path(&xdg.join("llama.cpp"));
    make_non_empty_path(&library_caches.join("llama.cpp"));

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.arg("scan")
        .arg(&xdg)
        .arg(&library_caches)
        .args(["--json", "--min-size", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"ruleId\": \"ai.llama_cpp_cache\"",
        ))
        .stdout(predicate::str::contains("\"safety\": \"report-only\""));
}

#[test]
fn whisper_cpp_and_comfyui_model_stores_are_report_only_with_markers() {
    let temp = TempDir::new().unwrap();
    let whisper_cpp = temp.path().join("whisper.cpp");
    let whisper_models = whisper_cpp.join("models");
    make_non_empty_path(&whisper_models);
    fs::write(
        whisper_models.join("download-ggml-model.sh"),
        "echo download",
    )
    .unwrap();

    let comfyui = temp.path().join("ComfyUI");
    let comfyui_models = comfyui.join("models");
    make_non_empty_path(&comfyui_models);
    fs::write(comfyui.join("folder_paths.py"), "").unwrap();
    fs::write(comfyui.join("main.py"), "").unwrap();

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        temp.path().to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "\"ruleId\": \"ai.whisper_cpp_models\"",
    ))
    .stdout(predicate::str::contains(
        "\"ruleId\": \"ai.comfyui_models\"",
    ))
    .stdout(predicate::str::contains("\"safety\": \"report-only\""));
}

#[test]
fn ai_cache_names_outside_exact_anchors_are_not_classified() {
    let temp = TempDir::new().unwrap();
    make_dir(temp.path(), "torch_compile_cache");
    make_dir(temp.path(), "whisper");
    make_dir(temp.path(), "llama.cpp");
    make_non_empty_path(&temp.path().join("whisper.cpp").join("models"));
    make_non_empty_path(&temp.path().join("ComfyUI").join("models"));

    let mut cmd = Command::cargo_bin("rclean").unwrap();
    cmd.args([
        "scan",
        temp.path().to_str().unwrap(),
        "--json",
        "--min-size",
        "0",
    ])
    .assert()
    .code(3)
    .stdout(predicate::str::contains("\"ruleId\": \"ai.vllm_compile_cache\"").not())
    .stdout(predicate::str::contains("\"ruleId\": \"ai.whisper_models\"").not())
    .stdout(predicate::str::contains("\"ruleId\": \"ai.llama_cpp_cache\"").not())
    .stdout(predicate::str::contains("\"ruleId\": \"ai.whisper_cpp_models\"").not())
    .stdout(predicate::str::contains("\"ruleId\": \"ai.comfyui_models\"").not());
}
