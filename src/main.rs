mod clean;
mod cli;
mod model;
mod output;
mod parse;
mod rules;
mod scan;

use std::process::ExitCode;

use clap::Parser;
use cli::{Cli, Commands};
use model::Safety;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan(args) => {
            let options = args.to_scan_options()?;
            let report = scan::scan(&args.paths_or_current_dir(), &options)?;
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
            let options = args.common.to_scan_options()?;
            let report = scan::scan(&args.common.paths_or_current_dir(), &options)?;
            let selected = clean::select_candidates(&report, &args)?;

            if args.common.json {
                output::print_json(&report)?;
            } else {
                output::print_table(&report);
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
    }
}
