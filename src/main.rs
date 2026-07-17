mod agent;
mod clean;
mod cli;
mod docker;
mod doctor;
mod error;
mod free;
#[cfg(feature = "graveyard")]
mod graveyard;
mod model;
mod output;
mod parse;
mod path_util;
mod plan;
mod rules;
mod scan;
mod stamp;
mod stdio;
#[cfg(test)]
mod test_support;
#[cfg(feature = "tui")]
mod tui;
mod user_rules;
mod watch;

use std::path::{Path, PathBuf};
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

    let Some(command) = cli.command else {
        return run_default_interactive();
    };

    match command {
        Commands::Agent(args) => match args.command {
            cli::AgentCommands::Doctor(doctor_args) => {
                let report = agent::diagnose_agent(doctor_args.tool);
                let output_result = if doctor_args.json {
                    let json = serde_json::to_string_pretty(&report)
                        .map_err(error::RcleanError::Output)?;
                    stdio::write_line(format_args!("{json}")).map_err(Into::into)
                } else {
                    output::print_agent_report(&report)
                };
                stdio::finish_output(ExitCode::SUCCESS, output_result)
            }
            cli::AgentCommands::Optimize(optimize_args) => {
                let result = agent::optimize(agent::OptimizeOptions {
                    tool: optimize_args.tool,
                    disable_auto_update: optimize_args.disable_auto_update,
                    apply: optimize_args.yes,
                    codex_defaults_domain: optimize_args.defaults_domain,
                })?;
                let output_result = if optimize_args.json {
                    let json = serde_json::to_string_pretty(&result)
                        .map_err(error::RcleanError::Output)?;
                    stdio::write_line(format_args!("{json}")).map_err(Into::into)
                } else {
                    output::print_agent_optimize_result(&result)
                };
                stdio::finish_output(ExitCode::SUCCESS, output_result)
            }
        },
        Commands::Docker(args) => match args.command {
            cli::DockerCommands::Report(report_args) => {
                let timeout = parse::parse_timeout_duration(&report_args.timeout)?;
                let report = docker::report(docker::DockerReportOptions {
                    timeout,
                    ..docker::DockerReportOptions::default()
                });
                let available = report.status.is_available();
                let output_result = if report_args.json {
                    let json = serde_json::to_string_pretty(&report)
                        .map_err(error::RcleanError::Output)?;
                    stdio::write_line(format_args!("{json}")).map_err(Into::into)
                } else {
                    docker::print_report(&report)
                };
                let status = if available {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(3)
                };
                stdio::finish_output(status, output_result)
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
            let output_result = if args.json {
                output::print_json(&report)
            } else {
                output::print_table(&report)
            };
            let status = if report.summary.candidates == 0 {
                ExitCode::from(3)
            } else {
                ExitCode::SUCCESS
            };
            stdio::finish_output(status, output_result)
        }
        Commands::Clean(args) => run_clean(args),
        Commands::Free(args) => free::run(args),
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
        Commands::Stamp(args) => stamp::run(args),
        Commands::Explain(args) => {
            let explanation =
                scan::explain_path_with_activity_depth(&args.path, args.activity_depth)?;
            let status = match explanation.safety {
                // ReportOnly shares the "refuse to clean" semantic with
                // Blocked; surface the same exit code so callers don't
                // have to distinguish unless they want to.
                Safety::Blocked | Safety::ReportOnly => ExitCode::from(4),
                Safety::Unknown => ExitCode::from(3),
                Safety::Safe | Safety::Caution => ExitCode::SUCCESS,
            };
            stdio::finish_output(status, output::print_explanation(&explanation))
        }
        Commands::Rules => stdio::finish_output(ExitCode::SUCCESS, output::print_rules()),
        Commands::Completions(args) => {
            use clap::CommandFactory;
            let mut buffer = Vec::new();
            clap_complete::generate(args.shell, &mut Cli::command(), "rclean", &mut buffer);
            stdio::finish_output(
                ExitCode::SUCCESS,
                stdio::write_bytes(&buffer).map_err(Into::into),
            )
        }
        Commands::Man => {
            use clap::CommandFactory;
            let man = clap_mangen::Man::new(Cli::command());
            let mut buffer = Vec::new();
            man.render(&mut buffer).map_err(|err| {
                RcleanError::Clean(crate::error::CleanError::Generic(format!(
                    "failed to render man page: {err}"
                )))
            })?;
            stdio::finish_output(
                ExitCode::SUCCESS,
                stdio::write_bytes(&buffer).map_err(Into::into),
            )
        }
        Commands::Doctor(args) => {
            let report = if args.docker {
                doctor::diagnose_with_options(doctor::DoctorOptions {
                    include_docker: true,
                })
            } else {
                doctor::diagnose()
            };
            let status = if report.applicable_count() == 0 {
                ExitCode::from(3)
            } else {
                ExitCode::SUCCESS
            };
            stdio::finish_output(status, output::print_doctor(&report))
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
                let output_result = if list_args.json {
                    let json = serde_json::to_string_pretty(&records)
                        .map_err(error::RcleanError::Output)?;
                    stdio::write_line(format_args!("{json}")).map_err(Into::into)
                } else {
                    output::print_graveyard_list(&records)
                };
                let status = if records.is_empty() {
                    ExitCode::from(3)
                } else {
                    ExitCode::SUCCESS
                };
                stdio::finish_output(status, output_result)
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

/// The no-subcommand entry point (spec: docs/specs/v0.2-best-ux.md §3.2 B2):
/// scan the current directory, select interactively, and delete
/// recoverably. Without a human at a terminal this must never reach a
/// destructive path — agents and scripts use explicit subcommands.
fn run_default_interactive() -> Result<ExitCode, RcleanError> {
    use std::io::IsTerminal;

    if !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
        use clap::CommandFactory;
        let mut buffer = Vec::new();
        Cli::command().write_help(&mut buffer)?;
        return stdio::finish_output(
            ExitCode::from(2),
            stdio::write_bytes(&buffer).map_err(Into::into),
        );
    }

    let cli = Cli::parse_from(DEFAULT_INTERACTIVE_ARGV);
    match cli.command {
        Some(Commands::Clean(args)) => run_clean(args),
        _ => Err(RcleanError::from(
            "internal error: default interactive argv did not parse as a clean command".to_string(),
        )),
    }
}

/// Argv equivalent of the default interactive flow. Built from the
/// stable CLI surface instead of hand-constructing `CleanArgs` so every
/// clap default (depth, min-size, ...) stays in one place.
#[cfg(all(feature = "tui", feature = "graveyard"))]
const DEFAULT_INTERACTIVE_ARGV: [&str; 4] = ["rclean", "clean", "--tui", "--graveyard"];
#[cfg(all(feature = "tui", not(feature = "graveyard")))]
const DEFAULT_INTERACTIVE_ARGV: [&str; 3] = ["rclean", "clean", "--tui"];
#[cfg(all(not(feature = "tui"), feature = "graveyard"))]
const DEFAULT_INTERACTIVE_ARGV: [&str; 3] = ["rclean", "clean", "--graveyard"];
#[cfg(all(not(feature = "tui"), not(feature = "graveyard")))]
const DEFAULT_INTERACTIVE_ARGV: [&str; 2] = ["rclean", "clean"];

fn run_clean(mut args: cli::CleanArgs) -> Result<ExitCode, RcleanError> {
    let action_plan = args
        .plan
        .as_deref()
        .map(plan::read_action_plan)
        .transpose()?;
    let delete_mode = if let Some(action_plan) = &action_plan {
        enforce_action_plan_delete_mode(&mut args, &action_plan.delete_mode)?;
        action_plan.delete_mode.clone()
    } else {
        requested_delete_mode(&args).to_string()
    };
    if !args.allow_broad_root {
        if let Some(action_plan) = &action_plan {
            let plan_roots: Vec<std::path::PathBuf> = action_plan
                .roots
                .iter()
                .map(std::path::PathBuf::from)
                .collect();
            clean::check_broad_roots(&plan_roots)?;
        } else {
            clean::check_broad_roots(&clean_roots_for_broad_check(&args)?)?;
        }
    }
    let (selected, report) = if let Some(action_plan) = &action_plan {
        let selected = plan::selected_from_action_plan(action_plan)?;
        plan::revalidate_selected(action_plan, &selected)?;
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
                &delete_mode,
            )?;
            // User-facing success confirmation. Bypass the tracing
            // filter (default `warn` would hide info!) so the message
            // stays visible without --verbose, matching v0.1.0.
            write_stderr_line(format_args!("wrote action plan: {}", plan_path.display()))?;
        }
        let selected = clean::select_candidates(&report, &args)?;
        (selected, Some(report))
    };
    let pre_delete_status = if selected.is_empty() {
        ExitCode::from(3)
    } else {
        ExitCode::SUCCESS
    };

    if let Some(report) = &report {
        let output_result = if args.common.json {
            output::print_json(report)
        } else {
            output::print_table(report)
        };
        if !stdio::continue_after_output(output_result)? {
            return Ok(pre_delete_status);
        }
    }
    if !args.common.json
        && !stdio::continue_after_output(clean::print_plan(&selected, &delete_mode, args.dry_run))?
    {
        return Ok(pre_delete_status);
    }

    if selected.is_empty() {
        return Ok(ExitCode::from(3));
    }

    if args.dry_run {
        return Ok(ExitCode::SUCCESS);
    }

    if let Some(audit_log) = args.audit_log.as_deref() {
        clean::validate_audit_log_path(audit_log, &selected)?;
    }
    clean::confirm_if_needed(&selected, &args)?;
    let mut audit_logger = args
        .audit_log
        .as_deref()
        .map(clean::DeleteAuditLogger::new)
        .transpose()?;
    #[cfg(feature = "graveyard")]
    let result = if args.graveyard {
        // SPEC §4.7.1: lazy create on first bury.
        let yard = graveyard::Graveyard::open(graveyard::default_root());
        clean::delete_selected_into_graveyard(&selected, &yard, audit_logger.as_mut())?
    } else {
        clean::delete_selected(&selected, args.permanent, audit_logger.as_mut())?
    };
    #[cfg(not(feature = "graveyard"))]
    let result = clean::delete_selected(&selected, args.permanent, audit_logger.as_mut())?;
    let status = if result.failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    };
    let output_result = clean::print_clean_result(&result).and_then(|()| {
        if args.common.json {
            Ok(())
        } else {
            clean::print_recovery_summary(&result, &delete_mode)
        }
    });
    stdio::finish_output(status, output_result)
}

fn clean_roots_for_broad_check(args: &cli::CleanArgs) -> Result<Vec<PathBuf>, RcleanError> {
    let roots = args.common.paths_or_current_dir();
    if !args.common.tmp || std::env::var_os("RCLEAN_TMP_ROOTS").is_some() {
        return Ok(roots);
    }

    roots
        .into_iter()
        .filter_map(|root| match is_builtin_tmp_root(&root) {
            Ok(true) => None,
            Ok(false) => Some(Ok(root)),
            Err(err) => Some(Err(err)),
        })
        .collect()
}

fn is_builtin_tmp_root(path: &Path) -> Result<bool, RcleanError> {
    let canonical = path.canonicalize().map_err(|source| {
        RcleanError::Clean(crate::error::CleanError::Generic(format!(
            "failed to canonicalize temp root {}: {source}",
            path.display()
        )))
    })?;

    #[cfg(target_os = "macos")]
    let roots = [Path::new("/private/tmp"), Path::new("/tmp")];
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let roots = [Path::new("/tmp")];
    #[cfg(target_os = "windows")]
    let roots: [&Path; 0] = [];

    Ok(roots.iter().any(|root| {
        root.canonicalize()
            .map(|builtin| builtin == canonical)
            .unwrap_or_else(|_| *root == canonical)
    }))
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

fn requested_explicit_delete_mode(args: &cli::CleanArgs) -> Option<&'static str> {
    if args.permanent {
        return Some("permanent");
    }
    #[cfg(feature = "graveyard")]
    if args.graveyard {
        return Some("graveyard");
    }
    None
}

fn enforce_action_plan_delete_mode(
    args: &mut cli::CleanArgs,
    plan_delete_mode: &str,
) -> Result<(), RcleanError> {
    if let Some(cli_delete_mode) = requested_explicit_delete_mode(args)
        && cli_delete_mode != plan_delete_mode
    {
        return Err(error::CleanError::Generic(format!(
            "CLI delete mode {cli_delete_mode:?} conflicts with action plan deleteMode \
             {plan_delete_mode:?}; replay the plan without a mode flag or regenerate it with \
             the intended delete mode"
        ))
        .into());
    }

    match plan_delete_mode {
        "trash" => {
            args.permanent = false;
            #[cfg(feature = "graveyard")]
            {
                args.graveyard = false;
            }
            Ok(())
        }
        "permanent" => {
            args.permanent = true;
            #[cfg(feature = "graveyard")]
            {
                args.graveyard = false;
            }
            Ok(())
        }
        "graveyard" => {
            #[cfg(feature = "graveyard")]
            {
                args.permanent = false;
                args.graveyard = true;
                Ok(())
            }
            #[cfg(not(feature = "graveyard"))]
            {
                Err(error::CleanError::Generic(
                    "action plan deleteMode \"graveyard\" requires a build with the graveyard \
                     feature; rebuild with default features or regenerate the plan with deleteMode \
                     \"trash\" or \"permanent\""
                        .to_string(),
                )
                .into())
            }
        }
        other => Err(error::CleanError::Generic(format!(
            "unsupported action plan deleteMode {other:?}; expected trash, graveyard, or permanent"
        ))
        .into()),
    }
}

fn write_stderr_line(args: std::fmt::Arguments<'_>) -> Result<(), RcleanError> {
    use std::io::Write;

    let mut stderr = std::io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod default_flow_tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_graph_is_valid_for_active_features() {
        let command = Cli::command();
        command.clone().debug_assert();

        let clean = command
            .find_subcommand("clean")
            .expect("clean subcommand must exist");
        assert!(
            clean
                .get_arguments()
                .any(|argument| argument.get_id() == "permanent"),
            "--permanent must exist in every feature combination"
        );
        assert_eq!(
            clean
                .get_arguments()
                .any(|argument| argument.get_id() == "graveyard"),
            cfg!(feature = "graveyard"),
            "--graveyard presence must follow the graveyard feature"
        );

        assert!(
            Cli::try_parse_from(["rclean", "clean", "--permanent"]).is_ok(),
            "--permanent must parse in every feature combination"
        );

        #[cfg(feature = "graveyard")]
        {
            assert!(Cli::try_parse_from(["rclean", "clean", "--graveyard"]).is_ok());
            let conflict = Cli::try_parse_from(["rclean", "clean", "--permanent", "--graveyard"])
                .expect_err("delete-mode flags must remain mutually exclusive");
            assert_eq!(conflict.kind(), clap::error::ErrorKind::ArgumentConflict);
        }

        #[cfg(not(feature = "graveyard"))]
        {
            let missing = Cli::try_parse_from(["rclean", "clean", "--graveyard"])
                .expect_err("--graveyard must not exist without its feature");
            assert_eq!(missing.kind(), clap::error::ErrorKind::UnknownArgument);
        }
    }

    #[test]
    fn default_interactive_argv_parses_as_interactive_recoverable_clean() {
        let cli = Cli::parse_from(DEFAULT_INTERACTIVE_ARGV);
        let Some(Commands::Clean(args)) = cli.command else {
            panic!("default interactive argv must parse as a clean command");
        };
        assert!(!args.all);
        assert!(!args.dry_run);
        assert!(!args.permanent);
        assert!(args.plan.is_none());
        assert!(args.common.paths.is_empty());
        assert_eq!(args.tui, cfg!(feature = "tui"));
        #[cfg(feature = "graveyard")]
        assert!(args.graveyard);
    }
}
