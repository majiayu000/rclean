//! Static rule catalog data.
//!
//! Single source of truth for the user-facing `rclean rules` listing.
//! `classify_candidate` in `mod.rs` emits rule_ids that **must** all appear
//! in this slice — guarded by the `rules_lists_every_classifier_emitted_id`
//! integration test.

use super::RuleInfo;
use crate::model::Category;

pub(super) static RULES: &[RuleInfo] = &[
    RuleInfo {
        rule_id: "node.node_modules",
        category: Category::Deps,
        candidate: "node_modules",
        restore_hint: "Run npm install, pnpm install, yarn install, or bun install",
    },
    RuleInfo {
        rule_id: "node.next",
        category: Category::Build,
        candidate: ".next",
        restore_hint: "Run the Next.js build or dev command",
    },
    RuleInfo {
        rule_id: "node.turbo",
        category: Category::Cache,
        candidate: ".turbo",
        restore_hint: "Rebuilt by Turborepo",
    },
    RuleInfo {
        rule_id: "node.vite",
        category: Category::Cache,
        candidate: ".vite",
        restore_hint: "Rebuilt by Vite",
    },
    RuleInfo {
        rule_id: "node.parcel",
        category: Category::Cache,
        candidate: ".parcel-cache",
        restore_hint: "Rebuilt by Parcel",
    },
    RuleInfo {
        rule_id: "python.venv_dot",
        category: Category::Deps,
        candidate: ".venv",
        restore_hint: "Recreate the Python environment",
    },
    RuleInfo {
        rule_id: "python.venv_plain",
        category: Category::Deps,
        candidate: "venv",
        restore_hint: "Recreate the Python environment",
    },
    RuleInfo {
        rule_id: "python.pycache",
        category: Category::Cache,
        candidate: "__pycache__",
        restore_hint: "Recreated by Python",
    },
    RuleInfo {
        rule_id: "python.pytest",
        category: Category::Cache,
        candidate: ".pytest_cache",
        restore_hint: "Recreated by pytest",
    },
    RuleInfo {
        rule_id: "python.mypy",
        category: Category::Cache,
        candidate: ".mypy_cache",
        restore_hint: "Recreated by mypy",
    },
    RuleInfo {
        rule_id: "python.ruff",
        category: Category::Cache,
        candidate: ".ruff_cache",
        restore_hint: "Recreated by ruff",
    },
    RuleInfo {
        rule_id: "python.tox",
        category: Category::Cache,
        candidate: ".tox",
        restore_hint: "Recreated by tox",
    },
    RuleInfo {
        rule_id: "rust.target",
        category: Category::Build,
        candidate: "target",
        restore_hint: "Run cargo build or cargo test",
    },
    RuleInfo {
        rule_id: "go.vendor",
        category: Category::Deps,
        candidate: "vendor",
        restore_hint: "Run go mod vendor",
    },
    RuleInfo {
        rule_id: "ios.pods",
        category: Category::Deps,
        candidate: "Pods",
        restore_hint: "Run pod install",
    },
    RuleInfo {
        rule_id: "java.maven_target",
        category: Category::Build,
        candidate: "target",
        restore_hint: "Run Maven build",
    },
    RuleInfo {
        rule_id: "java.gradle_build",
        category: Category::Build,
        candidate: "build",
        restore_hint: "Run Gradle build",
    },
    RuleInfo {
        rule_id: "java.gradle_cache_local",
        category: Category::Cache,
        candidate: ".gradle",
        restore_hint: "Rebuilt by Gradle",
    },
    RuleInfo {
        rule_id: "dart.build",
        category: Category::Build,
        candidate: "build",
        restore_hint: "Run flutter build or dart build",
    },
    RuleInfo {
        rule_id: "dart.tool",
        category: Category::Cache,
        candidate: ".dart_tool",
        restore_hint: "Run flutter pub get or dart pub get",
    },
    RuleInfo {
        rule_id: "dotnet.bin",
        category: Category::Build,
        candidate: "bin",
        restore_hint: "Run dotnet build",
    },
    RuleInfo {
        rule_id: "dotnet.obj",
        category: Category::Build,
        candidate: "obj",
        restore_hint: "Run dotnet build",
    },
    RuleInfo {
        rule_id: "ruby.bundle",
        category: Category::Cache,
        candidate: ".bundle",
        restore_hint: "Run bundle install",
    },
    RuleInfo {
        rule_id: "ruby.vendor_bundle",
        category: Category::Deps,
        candidate: "vendor/bundle",
        restore_hint: "Run bundle install",
    },
    RuleInfo {
        rule_id: "generic.coverage",
        category: Category::Test,
        candidate: "coverage",
        restore_hint: "Re-run the test suite",
    },
    RuleInfo {
        rule_id: "node.build",
        category: Category::Build,
        candidate: "build",
        restore_hint: "Re-run the project build",
    },
    RuleInfo {
        rule_id: "node.dist",
        category: Category::Build,
        candidate: "dist",
        restore_hint: "Re-run the project build",
    },
    RuleInfo {
        rule_id: "node.out",
        category: Category::Build,
        candidate: "out",
        restore_hint: "Re-run the project build",
    },
    RuleInfo {
        rule_id: "xcode.derived_data",
        category: Category::Build,
        candidate: "DerivedData",
        restore_hint: "Xcode will repopulate it on the next build",
    },
    RuleInfo {
        rule_id: "cargo.registry_cache",
        category: Category::Cache,
        candidate: "cache",
        restore_hint: "Cargo will redownload crates on the next build",
    },
    RuleInfo {
        rule_id: "cargo.git_db",
        category: Category::Cache,
        candidate: "db",
        restore_hint: "Cargo will re-clone git dependencies on the next build",
    },
    RuleInfo {
        rule_id: "go.module_download_cache",
        category: Category::Cache,
        candidate: "download",
        restore_hint: "Go will redownload modules on the next build or test",
    },
    RuleInfo {
        rule_id: "go.build_cache",
        category: Category::Cache,
        candidate: "go-build",
        restore_hint: "Go will rebuild cached objects on the next build or test",
    },
    RuleInfo {
        rule_id: "node.npm_cacache",
        category: Category::Cache,
        candidate: "_cacache",
        restore_hint: "npm will rebuild the cache on the next install",
    },
    RuleInfo {
        rule_id: "node.pnpm_store",
        category: Category::Cache,
        candidate: "store",
        restore_hint: "pnpm will rebuild the store on the next install",
    },
    RuleInfo {
        rule_id: "node.yarn_cache",
        category: Category::Cache,
        candidate: "Yarn",
        restore_hint: "Yarn will rebuild the cache on the next install",
    },
    RuleInfo {
        rule_id: "js.deno_cache",
        category: Category::Cache,
        candidate: "deno",
        restore_hint: "Run `deno cache --reload`; Deno will refetch on the next run",
    },
    RuleInfo {
        rule_id: "pip.cache",
        category: Category::Cache,
        candidate: "pip",
        restore_hint: "pip will repopulate the cache on the next install",
    },
    RuleInfo {
        rule_id: "python.uv_cache",
        category: Category::Cache,
        candidate: "uv",
        restore_hint: "Run `uv cache clean`; uv will repopulate on the next sync",
    },
    RuleInfo {
        rule_id: "python.poetry_cache",
        category: Category::Cache,
        candidate: "pypoetry",
        restore_hint: "Poetry will repopulate the cache on the next install",
    },
    RuleInfo {
        rule_id: "python.pipx_cache",
        category: Category::Cache,
        candidate: "pipx",
        restore_hint: "pipx will repopulate the cache on the next `pipx run`",
    },
    RuleInfo {
        rule_id: "gradle.caches",
        category: Category::Cache,
        candidate: "caches",
        restore_hint: "Gradle will redownload dependencies on the next build",
    },
    RuleInfo {
        rule_id: "maven.local_repo",
        category: Category::Cache,
        candidate: "repository",
        restore_hint: "Maven will redownload dependencies on the next build",
    },
    RuleInfo {
        rule_id: "xcode.simulators",
        category: Category::Cache,
        candidate: "CoreSimulator",
        restore_hint: "Xcode will recreate simulators on the next iOS app run",
    },
    RuleInfo {
        rule_id: "bun.cache",
        category: Category::Cache,
        candidate: "cache",
        restore_hint: "bun will repopulate the cache on the next install",
    },
    RuleInfo {
        rule_id: "pre_commit.cache",
        category: Category::Cache,
        candidate: "pre-commit",
        restore_hint: "pre-commit will reinitialize hooks on the next run",
    },
    RuleInfo {
        rule_id: "playwright.browsers",
        category: Category::Cache,
        candidate: "ms-playwright",
        restore_hint: "Playwright will redownload browsers on next `npx playwright install`",
    },
    RuleInfo {
        rule_id: "app.shipit_caches",
        category: Category::Cache,
        candidate: "*.ShipIt",
        restore_hint: "none — these are leftover update packages from completed app updates",
    },
    RuleInfo {
        rule_id: "chrome.cache",
        category: Category::Cache,
        candidate: "Chrome",
        restore_hint: "Chrome will repopulate the cache on next browsing",
    },
    RuleInfo {
        rule_id: "chrome.google_updater",
        category: Category::Cache,
        candidate: "GoogleUpdater",
        restore_hint: "Chrome's updater will recreate it on next launch",
    },
];
