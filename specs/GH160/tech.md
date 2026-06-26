# GH160 technical spec: conservative developer cache rules

## Rule Contract

Every rule added for GH160 must define:

- stable rule id
- exact path or versioned product anchor
- safety tier and restore hint
- positive scan test
- negative scan test proving nearby user state is not matched

## Implemented Rule IDs

| Rule id | Match | Safety | Notes |
| --- | --- | --- | --- |
| `homebrew.downloads` | `~/Library/Caches/Homebrew/downloads` or `~/.cache/Homebrew/downloads` | safe | Direct deletion of downloaded archives only. |
| `dart.pub_hosted_cache` | `~/.pub-cache/hosted` | caution | Global package cache, redownload required. |
| `dart.pub_git_cache` | `~/.pub-cache/git` | caution | Global git dependency cache, reclone required. |
| `jetbrains.system_caches` | JetBrains product `caches` under system cache roots | caution | Exact versioned IDE product anchors only. |
| `jetbrains.logs` | JetBrains product logs under cache/log roots | caution | Excludes config, plugins, projects, and LocalHistory. |
| `android_studio.system_caches` | Android Studio product `caches` under Google cache roots | caution | Excludes SDK and AVD state. |
| `android_studio.logs` | Android Studio product logs under Google cache/log roots | caution | Excludes app support and roaming state. |

## Files

- Rule code: `src/rules/homebrew.rs`, `src/rules/dart_global.rs`,
  `src/rules/ide_caches.rs`
- Registration: `src/rules/mod.rs`, `src/rules/catalog.rs`,
  `src/rules/project.rs`
- Home roots: `src/cli.rs`
- Doctor output: `src/doctor.rs` and `src/doctor/anchors.rs`
- Tests: `tests/rules.rs`, `tests/cli.rs`, `tests/ide_caches_cli.rs`
- User docs: `README.md`

## Platform Roots

- macOS: `~/Library/Caches`, `~/Library/Logs`, `~/.cache`, `.pub-cache`
- Linux/XDG: `~/.cache`, `.pub-cache`
- Windows: `%LOCALAPPDATA%` equivalents under `AppData/Local`, plus `.cache`
  when relevant cache roots exist.

## Rejection Rules

- Do not classify Android SDK paths such as `~/Library/Android/sdk`.
- Do not classify AVD images under `.android/avd`.
- Do not classify IDE settings under `Application Support`, `.config`, or
  `.local/share`.
- Do not classify plugins, LocalHistory, projects, or arbitrary version-like
  paths outside the known vendor anchors.

## Verification

Focused verification:

```sh
cargo fmt -- --check
cargo test --test ide_caches_cli
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
