use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::agent::AgentTool;
use crate::model::Category;
use crate::parse::{parse_duration, parse_size};
use crate::scan::ScanOptions;

#[derive(Debug, Parser)]
#[command(name = "rclean")]
#[command(about = "Find and clean rebuildable developer artifacts")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Diagnose and one-shot optimize local AI agent tools.
    Agent(AgentArgs),
    /// Scan for cleanable development artifacts. This never deletes files.
    Scan(CommonScanArgs),
    /// Clean selected artifacts after scanning.
    Clean(CleanArgs),
    /// Select cleanable artifacts in an interactive terminal UI.
    Tui(CommonScanArgs),
    /// Watch lockfiles and refresh cleanable artifact candidates.
    Watch(WatchArgs),
    /// Mark current artifacts, then sweep unchanged stamped artifacts into an ActionPlan.
    Stamp(StampArgs),
    /// Explain whether a single path is cleanable and why.
    Explain(ExplainArgs),
    /// Print the built-in cleanup rule catalog.
    Rules,
    /// Diagnostic: list which global-cache rules are applicable on
    /// this machine right now. Tells you which toolchain caches
    /// exist under $HOME without running a full scan.
    Doctor,
    /// Restore a grave from the rclean graveyard.
    #[cfg(feature = "graveyard")]
    Restore(RestoreArgs),
    /// Inspect or maintain the rclean graveyard.
    #[cfg(feature = "graveyard")]
    Graveyard(GraveyardArgs),
}

#[derive(Debug, Args)]
pub struct AgentArgs {
    #[command(subcommand)]
    pub command: AgentCommands,
}

#[derive(Debug, Subcommand)]
pub enum AgentCommands {
    /// Report local process, disk, power, and update signals for an agent tool.
    Doctor(AgentDoctorArgs),
    /// Apply explicit one-shot settings for an agent tool.
    Optimize(AgentOptimizeArgs),
}

#[derive(Debug, Args)]
pub struct AgentDoctorArgs {
    /// Agent tool to inspect.
    #[arg(value_enum, default_value_t = AgentTool::Codex)]
    pub tool: AgentTool,

    /// Emit machine-readable JSON instead of a table.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct AgentOptimizeArgs {
    /// Agent tool to optimize.
    #[arg(value_enum, default_value_t = AgentTool::Codex)]
    pub tool: AgentTool,

    /// Disable app-managed automatic update checks where supported.
    #[arg(long)]
    pub disable_auto_update: bool,

    /// Apply the selected changes. Without this, optimize is a dry run.
    #[arg(long)]
    pub yes: bool,

    /// Emit machine-readable JSON instead of text.
    #[arg(long)]
    pub json: bool,

    /// Internal test hook: override the macOS defaults domain.
    #[arg(long = "defaults-domain", hide = true)]
    pub defaults_domain: Option<String>,
}

#[cfg(feature = "graveyard")]
#[derive(Debug, Args)]
pub struct RestoreArgs {
    /// id of the grave to restore (from `rclean graveyard list`).
    #[arg(long = "id", value_name = "ID")]
    pub id: String,

    /// Restore to this path instead of the original. Useful when the
    /// original location is now occupied.
    #[arg(long, value_name = "PATH")]
    pub to: Option<PathBuf>,
}

#[cfg(feature = "graveyard")]
#[derive(Debug, Args)]
pub struct GraveyardArgs {
    #[command(subcommand)]
    pub command: GraveyardCommands,
}

#[cfg(feature = "graveyard")]
#[derive(Debug, Subcommand)]
pub enum GraveyardCommands {
    /// List active graves.
    List(GraveyardListArgs),
    /// Remove every grave past its expiry.
    Gc(GraveyardGcArgs),
}

#[cfg(feature = "graveyard")]
#[derive(Debug, Args)]
pub struct GraveyardListArgs {
    /// Emit machine-readable JSON instead of a table.
    #[arg(long)]
    pub json: bool,
}

#[cfg(feature = "graveyard")]
#[derive(Debug, Args)]
pub struct GraveyardGcArgs {
    /// Show what would be removed without actually deleting.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args, Clone)]
pub struct CommonScanArgs {
    /// Roots to scan. Defaults to the current directory.
    pub paths: Vec<PathBuf>,

    /// Emit machine-readable JSON.
    #[arg(long)]
    pub json: bool,

    /// Show skipped paths and policy reasons.
    #[arg(long)]
    pub verbose: bool,

    /// Max traversal depth from each root.
    #[arg(long, default_value_t = 6)]
    pub depth: usize,

    /// Minimum candidate size. Examples: 0, 100mb, 1g.
    #[arg(long, default_value = "1mb")]
    pub min_size: String,

    /// Only include projects whose activity is older than this duration.
    #[arg(long)]
    pub older_than: Option<String>,

    /// Include only these categories: deps,build,cache,test.
    #[arg(long, value_delimiter = ',')]
    pub category: Vec<String>,

    /// Include only these rule ids.
    #[arg(long, value_delimiter = ',')]
    pub rule: Vec<String>,

    /// Include caution candidates in bulk selection.
    #[arg(long)]
    pub include_caution: bool,

    /// Include blocked candidates in reports.
    #[arg(long)]
    pub include_blocked: bool,

    /// Write scan results as an auditable action plan.
    #[arg(long)]
    pub write_plan: Option<PathBuf>,

    /// Exclude paths from scan. Uses .gitignore-style globs. Repeatable.
    /// Layered on top of any `.rcleanignore` file at the scan root.
    #[arg(long = "ignore", value_name = "GLOB")]
    pub ignore: Vec<String>,

    /// Expand to all developer toolchain cache locations under $HOME
    /// (~/.cargo, ~/go, ~/.gradle, ~/.m2, ~/.npm, ~/.pnpm-store,
    /// plus platform-specific paths like ~/Library/Caches,
    /// ~/Library/pnpm, ~/Library/Developer, and
    /// ~/Library/Application Support/Google on macOS, ~/.cache and
    /// ~/.local/share/pnpm on Linux). Conflicts with positional `paths`.
    ///
    /// This is the entry point for the v0.2 "developer-grade mole"
    /// flow — it activates the global cache rules
    /// (xcode.derived_data, cargo.registry_cache, go.build_cache,
    /// go.module_download_cache, gradle.caches, maven.local_repo,
    /// node.npm_cacache, node.pnpm_store, node.yarn_cache,
    /// pip.cache, xcode.simulators) without forcing the user to
    /// remember every path.
    #[arg(long, conflicts_with = "paths")]
    pub home: bool,
}

#[derive(Debug, Args)]
pub struct CleanArgs {
    #[command(flatten)]
    pub common: CommonScanArgs,

    /// Select all safe candidates after filters.
    #[arg(long)]
    pub all: bool,

    /// Show deletion plan without deleting.
    #[arg(long)]
    pub dry_run: bool,

    /// Permanently delete selected candidates.
    #[arg(long, conflicts_with = "graveyard")]
    pub permanent: bool,

    /// Move selected candidates into the rclean graveyard (7-day
    /// recoverable). Mutually exclusive with `--permanent`. Requires
    /// the `graveyard` feature (default on).
    #[cfg(feature = "graveyard")]
    #[arg(long)]
    pub graveyard: bool,

    /// Skip confirmation prompts where allowed.
    #[arg(long)]
    pub yes: bool,

    /// Read selected candidates from a previously written action plan.
    #[arg(long)]
    pub plan: Option<PathBuf>,

    /// Use the feature-gated terminal selector instead of numbered text prompts.
    #[arg(long)]
    pub tui: bool,

    /// Allow cleaning when a scan root resolves to a broad system or user root
    /// (for example /, $HOME, /etc, /usr). Off by default.
    #[arg(long)]
    pub allow_broad_root: bool,
}

#[derive(Debug, Args)]
pub struct WatchArgs {
    #[command(flatten)]
    pub common: CommonScanArgs,

    /// Poll interval after the watcher is idle or unavailable. Examples: 60s, 5m.
    #[arg(long, default_value = "60s")]
    pub every: String,
}

#[derive(Debug, Args)]
pub struct StampArgs {
    #[command(flatten)]
    pub common: CommonScanArgs,

    /// Write an ActionPlan for stamped candidates that have not changed since stamping.
    #[arg(long)]
    pub sweep: bool,
}

#[derive(Debug, Args)]
pub struct ExplainArgs {
    pub path: PathBuf,
}

impl CommonScanArgs {
    pub fn paths_or_current_dir(&self) -> Vec<PathBuf> {
        if self.home {
            return home_toolchain_paths();
        }
        if self.paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            self.paths.clone()
        }
    }

    pub fn to_scan_options(&self) -> Result<ScanOptions, crate::error::ParseError> {
        let categories = if self.category.is_empty() {
            None
        } else {
            let mut parsed = Vec::new();
            for raw in &self.category {
                parsed.push(raw.parse::<Category>()?);
            }
            Some(parsed)
        };

        Ok(ScanOptions {
            max_depth: self.depth,
            min_size: parse_size(&self.min_size)?,
            older_than: self.older_than.as_deref().map(parse_duration).transpose()?,
            categories,
            rule_ids: if self.rule.is_empty() {
                None
            } else {
                Some(self.rule.clone())
            },
            include_blocked: self.include_blocked,
            verbose: self.verbose,
            ignore_globs: self.ignore.clone(),
        })
    }
}

/// Roots `--home` expands into. Only paths that actually exist on
/// disk are returned, so a user without (e.g.) Maven installed
/// won't see a noisy "cannot canonicalize ~/.m2" error.
///
/// Order is deterministic so the report ordering is stable.
fn home_toolchain_paths() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME") else {
        return Vec::new();
    };
    let home = PathBuf::from(home);

    let mut candidates: Vec<PathBuf> = vec![
        home.join(".cargo"),
        home.join("go"),
        home.join(".gradle"),
        home.join(".m2"),
        home.join(".npm"),
        home.join(".pnpm-store"),
        home.join(".ollama"),
        home.join(".bun"),
        home.join(".bundle"),
        home.join(".kube"),
        home.join(".config").join("gcloud"),
        home.join(".vscode").join("extensions"),
        home.join(".cursor").join("extensions"),
        home.join(".local")
            .join("share")
            .join("claude")
            .join("versions"),
    ];
    if let Some(gopath) = std::env::var_os("GOPATH") {
        candidates.extend(std::env::split_paths(&gopath));
    }

    #[cfg(target_os = "macos")]
    {
        candidates.push(home.join("Library").join("Caches"));
        candidates.push(home.join("Library").join("pnpm"));
        candidates.push(home.join("Library").join("Developer"));
        candidates.push(
            home.join("Library")
                .join("Application Support")
                .join("Google"),
        );
        for app in ["Code", "Cursor", "Notion", "Slack", "LarkInternational"] {
            candidates.push(home.join("Library").join("Application Support").join(app));
        }
        // Some global tools use XDG-style caches on macOS instead of
        // `~/Library/Caches` (for example pre-commit and uv).
        candidates.push(home.join(".cache"));
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        candidates.push(home.join(".cache"));
        candidates.push(home.join(".local").join("share").join("pnpm"));
    }

    #[cfg(target_os = "windows")]
    {
        let local_app_data = home.join("AppData").join("Local");
        let xdg_cache = home.join(".cache");
        if ["huggingface", "pre-commit", "puppeteer", "torch"]
            .iter()
            .any(|name| xdg_cache.join(name).is_dir())
        {
            candidates.push(xdg_cache);
        }
        // The walker classifies child directories, so `go-build`
        // needs its parent as the scan root. Keep pnpm targeted when
        // the Go build cache is absent.
        if local_app_data.join("go-build").is_dir() {
            candidates.push(local_app_data);
        } else {
            candidates.push(local_app_data.join("pnpm"));
        }
    }

    candidates.retain(|p| p.is_dir());
    candidates.sort();
    candidates.dedup();
    candidates
}
