use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

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
    /// Scan for cleanable development artifacts. This never deletes files.
    Scan(CommonScanArgs),
    /// Clean selected artifacts after scanning.
    Clean(CleanArgs),
    /// Select cleanable artifacts in an interactive terminal UI.
    Tui(CommonScanArgs),
    /// Watch lockfiles and refresh cleanable artifact candidates.
    Watch(WatchArgs),
    /// Explain whether a single path is cleanable and why.
    Explain(ExplainArgs),
    /// Print the built-in cleanup rule catalog.
    Rules,
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
pub struct ExplainArgs {
    pub path: PathBuf,
}

impl CommonScanArgs {
    pub fn paths_or_current_dir(&self) -> Vec<PathBuf> {
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
