mod agent;
mod clean;
mod cli;
mod doctor;
mod error;
#[cfg(feature = "graveyard")]
mod graveyard;
mod model;
mod output;
mod parse;
mod plan;
mod rules;
mod scan;
#[cfg(feature = "tui")]
mod tui;
mod user_rules;
mod watch;

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
        .with_level(false)
        .with_target(false)
        .without_time()
        .with_writer(std::io::stderr)
        .try_init();
}

fn run() -> Result<ExitCode, RcleanError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Agent(args) => match args.command {
            cli::AgentCommands::Doctor(doctor_args) => {
                let report = agent::diagnose_agent(doctor_args.tool);
                if doctor_args.json {
                    let json = serde_json::to_string_pretty(&report)
                        .map_err(error::RcleanError::Output)?;
                    println!("{json}");
                } else {
                    output::print_agent_report(&report);
                }
                Ok(ExitCode::SUCCESS)
            }
            cli::AgentCommands::Optimize(optimize_args) => {
                let result = agent::optimize(agent::OptimizeOptions {
                    tool: optimize_args.tool,
                    disable_auto_update: optimize_args.disable_auto_update,
                    apply: optimize_args.yes,
                    codex_defaults_domain: optimize_args.defaults_domain,
                })?;
                if optimize_args.json {
                    let json = serde_json::to_string_pretty(&result)
                        .map_err(error::RcleanError::Output)?;
                    println!("{json}");
                } else {
                    output::print_agent_optimize_result(&result);
                }
                Ok(ExitCode::SUCCESS)
            }
        },
        Commands::Scan(args) => {
            let options = args.to_scan_options()?;
            let report = scan::scan(&args.paths_or_current_dir(), &options)?;
            if let Some(plan_path) = &args.write_plan {
                plan::write_action_plan(&report, plan_path, args.include_caution, false, "trash")?;
                // User-facing success confirmation. Bypass the tracing
                // filter (default `warn` would hide info!) so the message
                // stays visible without --verbose, matching v0.1.0.
                write_stderr_line(format_args!("wrote action plan: {}", plan_path.display()))?;
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
            let delete_mode = requested_delete_mode(&args);
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
                        delete_mode,
                    )?;
                    // User-facing success confirmation. Bypass the tracing
                    // filter (default `warn` would hide info!) so the message
                    // stays visible without --verbose, matching v0.1.0.
                    write_stderr_line(format_args!("wrote action plan: {}", plan_path.display()))?;
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
                clean::print_plan(&selected, delete_mode, args.dry_run);
            }

            if selected.is_empty() {
                return Ok(ExitCode::from(3));
            }

            if args.dry_run {
                return Ok(ExitCode::SUCCESS);
            }

            clean::confirm_if_needed(&selected, &args)?;
            #[cfg(feature = "graveyard")]
            let result = if args.graveyard {
                // SPEC §4.7.1: lazy create on first bury.
                let yard = graveyard::Graveyard::open(graveyard::default_root());
                clean::delete_selected_into_graveyard(&selected, &yard)?
            } else {
                clean::delete_selected(&selected, args.permanent)?
            };
            #[cfg(not(feature = "graveyard"))]
            let result = clean::delete_selected(&selected, args.permanent)?;
            clean::print_clean_result(&result);

            if result.failed.is_empty() {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::from(1))
            }
        }
        Commands::Tui(args) => {
            #[cfg(feature = "tui")]
            {
                tui::run_command(args)
            }
            #[cfg(not(feature = "tui"))]
            {
                let _ = args;
                Err(RcleanError::from(
                    "TUI support is not enabled in this build; rebuild with --features tui"
                        .to_string(),
                ))
            }
        }
        Commands::Watch(args) => watch::run(args),
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
        #[cfg(feature = "graveyard")]
        Commands::Restore(args) => {
            let yard = graveyard::Graveyard::open(graveyard::default_root());
            let record = yard.restore_by_id(&args.id, args.to.as_deref())?;
            eprintln!(
                "restored {} -> {}",
                record.id,
                record.original_path.display()
            );
            Ok(ExitCode::SUCCESS)
        }
        #[cfg(feature = "graveyard")]
        Commands::Graveyard(args) => match args.command {
            cli::GraveyardCommands::List(list_args) => {
                let yard = graveyard::Graveyard::open(graveyard::default_root());
                let records = yard.list()?;
                if list_args.json {
                    let json = serde_json::to_string_pretty(&records)
                        .map_err(error::RcleanError::Output)?;
                    println!("{json}");
                } else {
                    output::print_graveyard_list(&records);
                }
                if records.is_empty() {
                    Ok(ExitCode::from(3))
                } else {
                    Ok(ExitCode::SUCCESS)
                }
            }
            cli::GraveyardCommands::Gc(gc_args) => {
                let yard = graveyard::Graveyard::open(graveyard::default_root());
                let collected = yard.gc(gc_args.dry_run)?;
                if gc_args.dry_run {
                    eprintln!("dry-run: would remove {} expired grave(s)", collected.len());
                } else {
                    eprintln!("removed {} expired grave(s)", collected.len());
                }
                Ok(ExitCode::SUCCESS)
            }
        },
    }
}

fn requested_delete_mode(args: &cli::CleanArgs) -> &'static str {
    if args.permanent {
        return "permanent";
    }
    #[cfg(feature = "graveyard")]
    if args.graveyard {
        return "graveyard";
    }
    "trash"
}

fn write_stderr_line(args: std::fmt::Arguments<'_>) -> Result<(), RcleanError> {
    use std::io::Write;

    let mut stderr = std::io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")?;
    Ok(())
}
