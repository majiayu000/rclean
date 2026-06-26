use std::path::{Path, PathBuf};

use crate::model::{CandidateDraft, Category};

mod agent_tmp;
mod ai_models;
mod android_sdk;
mod app_caches;
mod browser_global;
mod bun;
mod cargo_global;
mod catalog;
mod dart_global;
mod dotnet;
mod flutter;
mod generic;
mod go;
mod gradle;
mod homebrew;
mod ide_caches;
mod ios;
mod js_global;
mod jvm;
mod macos_system;
mod markers;
mod maven;
mod node;
mod node_global;
mod pip;
mod playwright;
mod pre_commit;
mod project;
mod python;
mod python_global;
mod ruby;
mod rust;
mod user_tool_caches;
mod xcode;

pub use agent_tmp::classify as classify_agent_tmp_worktree;
pub use project::{detect_project_kind, is_candidate_name, is_global_rule, is_project_marker_name};

use markers::is_shared_cargo_target;

#[derive(Debug, Clone)]
pub struct RuleInfo {
    pub rule_id: &'static str,
    pub category: Category,
    pub candidate: &'static str,
    pub restore_hint: &'static str,
}

pub fn rule_catalog() -> Vec<RuleInfo> {
    catalog::RULES.to_vec()
}

pub fn system_scan_paths() -> Vec<PathBuf> {
    macos_system::system_scan_paths()
}

pub struct ClassifyContext<'a> {
    pub project_dir: &'a Path,
    pub name: &'a str,
    pub path: &'a Path,
}

pub trait RuleSet: Sync {
    fn classify(&self, context: &ClassifyContext<'_>) -> Option<CandidateDraft>;
}

struct RuleFn(fn(&Path, &str, &Path) -> Option<CandidateDraft>);

impl RuleSet for RuleFn {
    fn classify(&self, context: &ClassifyContext<'_>) -> Option<CandidateDraft> {
        (self.0)(context.project_dir, context.name, context.path)
    }
}

static RUST_RULES: RuleFn = RuleFn(rust::classify);
static JVM_RULES: RuleFn = RuleFn(jvm::classify);
static FLUTTER_RULES: RuleFn = RuleFn(flutter::classify);
static NODE_RULES: RuleFn = RuleFn(node::classify);
static PYTHON_RULES: RuleFn = RuleFn(python::classify);
static DOTNET_RULES: RuleFn = RuleFn(dotnet::classify);
static RUBY_RULES: RuleFn = RuleFn(ruby::classify);
static GO_RULES: RuleFn = RuleFn(go::classify);
static IOS_RULES: RuleFn = RuleFn(ios::classify);
static XCODE_RULES: RuleFn = RuleFn(xcode::classify);
static CARGO_GLOBAL_RULES: RuleFn = RuleFn(cargo_global::classify);
static HOMEBREW_RULES: RuleFn = RuleFn(homebrew::classify);
static ANDROID_SDK_RULES: RuleFn = RuleFn(android_sdk::classify);
static IDE_CACHES_RULES: RuleFn = RuleFn(ide_caches::classify);
static DART_GLOBAL_RULES: RuleFn = RuleFn(dart_global::classify);
static NODE_GLOBAL_RULES: RuleFn = RuleFn(node_global::classify);
static AI_MODELS_RULES: RuleFn = RuleFn(ai_models::classify);
static BROWSER_GLOBAL_RULES: RuleFn = RuleFn(browser_global::classify);
static JS_GLOBAL_RULES: RuleFn = RuleFn(js_global::classify);
static MACOS_SYSTEM_RULES: RuleFn = RuleFn(macos_system::classify);
static PIP_RULES: RuleFn = RuleFn(pip::classify);
static PYTHON_GLOBAL_RULES: RuleFn = RuleFn(python_global::classify);
static GRADLE_RULES: RuleFn = RuleFn(gradle::classify);
static MAVEN_RULES: RuleFn = RuleFn(maven::classify);
static BUN_RULES: RuleFn = RuleFn(bun::classify);
static PRE_COMMIT_RULES: RuleFn = RuleFn(pre_commit::classify);
static PLAYWRIGHT_RULES: RuleFn = RuleFn(playwright::classify);
static APP_CACHES_RULES: RuleFn = RuleFn(app_caches::classify);
static USER_TOOL_CACHES_RULES: RuleFn = RuleFn(user_tool_caches::classify);
static GENERIC_RULES: RuleFn = RuleFn(generic::classify);

static BUILTIN_RULES: [&dyn RuleSet; 30] = [
    &RUST_RULES,
    &JVM_RULES,
    &FLUTTER_RULES,
    &NODE_RULES,
    &PYTHON_RULES,
    &DOTNET_RULES,
    &RUBY_RULES,
    &GO_RULES,
    &IOS_RULES,
    &XCODE_RULES,
    &CARGO_GLOBAL_RULES,
    &HOMEBREW_RULES,
    &ANDROID_SDK_RULES,
    &IDE_CACHES_RULES,
    &DART_GLOBAL_RULES,
    &NODE_GLOBAL_RULES,
    &AI_MODELS_RULES,
    &BROWSER_GLOBAL_RULES,
    &JS_GLOBAL_RULES,
    &MACOS_SYSTEM_RULES,
    &PIP_RULES,
    &PYTHON_GLOBAL_RULES,
    &GRADLE_RULES,
    &MAVEN_RULES,
    &BUN_RULES,
    &PRE_COMMIT_RULES,
    &PLAYWRIGHT_RULES,
    &APP_CACHES_RULES,
    &USER_TOOL_CACHES_RULES,
    &GENERIC_RULES,
];

#[derive(Debug, Default, Clone, Copy)]
pub struct Classifier;

impl Classifier {
    pub fn classify(&self, project_dir: &Path, name: &str, path: &Path) -> Option<CandidateDraft> {
        let context = ClassifyContext {
            project_dir,
            name,
            path,
        };
        BUILTIN_RULES
            .iter()
            .find_map(|rule_set| rule_set.classify(&context))
    }
}

pub fn classify_candidate(project_dir: &Path, name: &str, path: PathBuf) -> Option<CandidateDraft> {
    let path_ref = path.as_path();
    // Order matches v0.1.0's match-arm priority. The ambiguous `build/`
    // directory name belongs to jvm (Gradle) and flutter (Dart) before
    // node — under a mixed Gradle+Node project, `build/` is a Gradle
    // output (Safety::Safe) and must not be reclassified as a Node
    // caution candidate. Same logic for `target/`: rust > jvm.
    Classifier
        .classify(project_dir, name, path_ref)
        .map(|mut draft| {
            if is_shared_cargo_target(project_dir, &draft.path) {
                draft.safety = crate::model::Safety::Blocked;
                draft
                    .warnings
                    .push("shared Cargo target directory detected".to_string());
            }
            draft
        })
}

pub fn allows_protected_user_data_path(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "macos.geod_map_tiles" | "macos.mediaanalysisd_cache" | "macos.mediaanalysisd_tmp"
    )
}

pub fn requires_sudo(rule_id: &str) -> bool {
    matches!(rule_id, "apple.idleassetsd")
}
