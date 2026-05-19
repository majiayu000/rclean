mod clean;
mod cli;
mod doctor;
mod error;
mod model;
mod output;
mod parse;
mod plan;
mod rules;
mod scan;
mod user_rules;

use std::process::ExitCode;

use clap::Parser;
use cli::{Cli, Commands};
use error::RcleanError;
use model::Safety;
use tracing::error;
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    init_tracing(verbose_flag());
    match run() {
        Ok(code) => code,
        Err(err) => {
            error!("{err}");
            ExitCode::from(1)
        }
    }
}

fn verbose_flag() -> bool {
    std::env::args().any(|a| a == "-v" || a == "--verbose")
}

fn init_tracing(verbose: bool) {
    let default_filter = if verbose { "debug" } else { "warn" };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .with_writer(std::io::stderr)
        .try_init();
}

fn run() -> Result<ExitCode, RcleanError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan(args) => {
            let options = args.to_scan_options()?;
            let report = scan::scan(&args.paths_or_current_dir(), &options)?;
            if let Some(plan_path) = &args.write_plan {
                plan::write_action_plan(&report, plan_path, args.include_caution, false, "trash")?;
                // User-facing success confirmation. Bypass the tracing
                // filter (default `warn` would hide info!) so the message
                // stays visible without --verbose, matching v0.1.0.
                eprintln!("wrote action plan: {}", plan_path.display());
            }
            if args.json {
                output::print_json(&report)?;
            } else {
                output::print_table(&report);
            }
            if report.summary.candidates == 0 {
                Ok(ExitCode::from(3))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        Commands::Clean(args) => {
            if !args.allow_broad_root {
                if let Some(plan_path) = &args.plan {
                    let action_plan = plan::read_action_plan(plan_path)?;
                    let plan_roots: Vec<std::path::PathBuf> = action_plan
                        .roots
                        .iter()
                        .map(std::path::PathBuf::from)
                        .collect();
                    clean::check_broad_roots(&plan_roots)?;
                } else {
                    clean::check_broad_roots(&args.common.paths_or_current_dir())?;
                }
            }
            let (selected, report) = if let Some(plan_path) = &args.plan {
                let action_plan = plan::read_action_plan(plan_path)?;
                let selected = plan::selected_from_action_plan(&action_plan)?;
                plan::revalidate_selected(&action_plan, &selected)?;
                (selected, None)
            } else {
                let options = args.common.to_scan_options()?;
                let report = scan::scan(&args.common.paths_or_current_dir(), &options)?;
                if let Some(plan_path) = &args.common.write_plan {
                    plan::write_action_plan(
                        &report,
                        plan_path,
                        args.common.include_caution,
                        args.permanent,
                        if args.permanent { "permanent" } else { "trash" },
                    )?;
                    // User-facing success confirmation. Bypass the tracing
                    // filter (default `warn` would hide info!) so the message
                    // stays visible without --verbose, matching v0.1.0.
                    eprintln!("wrote action plan: {}", plan_path.display());
                }
                let selected = clean::select_candidates(&report, &args)?;
                (selected, Some(report))
            };

            if let Some(report) = &report {
                if args.common.json {
                    output::print_json(report)?;
                } else {
                    output::print_table(report);
                }
            }
            if !args.common.json {
                clean::print_plan(&selected, args.permanent, args.dry_run);
            }

            if selected.is_empty() {
                return Ok(ExitCode::from(3));
            }

            if args.dry_run {
                return Ok(ExitCode::SUCCESS);
            }

            clean::confirm_if_needed(&selected, &args)?;
            let result = clean::delete_selected(&selected, args.permanent)?;
            clean::print_clean_result(&result);

            if result.failed.is_empty() {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::from(1))
            }
        }
        Commands::Explain(args) => {
            let explanation = scan::explain_path(&args.path)?;
            output::print_explanation(&explanation);
            match explanation.safety {
                Safety::Blocked => Ok(ExitCode::from(4)),
                Safety::Unknown => Ok(ExitCode::from(3)),
                Safety::Safe | Safety::Caution => Ok(ExitCode::SUCCESS),
            }
        }
        Commands::Rules => {
            output::print_rules();
            Ok(ExitCode::SUCCESS)
        }
        Commands::Doctor => {
            let report = doctor::diagnose();
            output::print_doctor(&report);
            if report.applicable_count() == 0 {
                Ok(ExitCode::from(3))
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
    }
}
