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
];
