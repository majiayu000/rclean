//! Per-ecosystem rule tests via `rclean scan --json`.
//!
//! These exercise `classify_candidate` end-to-end through the binary because
//! the crate has no `lib.rs`. Each test sets up a minimal project marker plus
//! the candidate dir, then asserts the JSON output names the expected rule_id.

#[path = "rules/ai_models.rs"]
mod ai_models;
#[path = "rules/common.rs"]
mod common;
#[path = "rules/platform_safety.rs"]
mod platform_safety;
#[path = "rules/project_artifacts.rs"]
mod project_artifacts;
#[path = "rules/tool_caches.rs"]
mod tool_caches;
