//! Diagnostics and one-shot optimization helpers for local AI agent tools.
//!
//! This module is deliberately separate from `scan` and `clean`: agent
//! settings are app/system state, not rebuildable developer artifacts.

mod codex;
#[cfg(any(target_os = "macos", test))]
mod macos;
mod process;
mod system;
mod types;

#[cfg(test)]
mod tests;

pub use codex::{diagnose_agent, optimize};
pub use types::{AgentError, AgentReport, AgentTool, OptimizeOptions, OptimizeResult};
