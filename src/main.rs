use clap::Parser;
use std::path::PathBuf;
use std::process::Termination;
use config::HookConfig;
use confirmation::ConfirmationOutcome;

mod check_submodules;
mod config;
mod confirmation;

/// Enum representing the overall program outcome
#[derive(Debug)]
enum ProgramOutcome {
    /// Successful outcome with confirmation result
    Success(ConfirmationOutcome),
    /// Submodule check error
    CheckError,
    /// No confirmation needed
    NoConfirmationNeeded,
}

impl Termination for ProgramOutcome {
    fn report(self) -> std::process::ExitCode {
        match self {
            ProgramOutcome::Success(outcome) => outcome.report(),
            ProgramOutcome::CheckError => std::process::ExitCode::from(3),
            ProgramOutcome::NoConfirmationNeeded => std::process::ExitCode::SUCCESS,
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Make failed checks hard errors
    #[arg(long)]
    strict: Option<bool>,
    /// Ask confirmation if commit contains a submodule update
    #[arg(long)]
    confirm_staging: Option<bool>,
    /// Ask confirmation if a submodule is modified and not staged for commit
    #[arg(long)]
    confirm_not_staging: Option<bool>,
    /// Repository path
    #[arg(long, default_value = ".")]
    repo: PathBuf,
}

fn main() -> ProgramOutcome {
    env_logger::init();
    let args = Args::parse();
    let cli_config = HookConfig {
        strict: args.strict,
        confirm_staging: args.confirm_staging,
        confirm_not_staging: args.confirm_not_staging,
    };
    let git_config = config::get_config();
    let strict = cli_config.strict.or(git_config.strict).unwrap_or(false);
    let confirm_staging = cli_config
        .confirm_staging
        .or(git_config.confirm_staging)
        .unwrap_or(true);
    let confirm_not_staging = cli_config
        .confirm_not_staging
        .or(git_config.confirm_not_staging)
        .unwrap_or(true);
    
    if confirm_staging || confirm_not_staging {
        // only check submodules if configuration enables confirmation
        match check_submodules::check_submodules(strict, args.repo.as_path()) {
            Ok(Some(diagnostics)) => {
                let prompt_for_confirmation = (!diagnostics.modified_not_staged_submodules.is_empty()
                    && confirm_not_staging)
                    || (!diagnostics.modified_staged_submodules.is_empty() && confirm_staging);
                
                if prompt_for_confirmation {
                    return match confirmation::ask_confirmation(&diagnostics) {
                        Ok(outcome) => {
                            match outcome {
                                ConfirmationOutcome::Confirmed => {
                                    // User confirmed
                                    ProgramOutcome::Success(ConfirmationOutcome::Confirmed)
                                }
                                ConfirmationOutcome::Declined => {
                                    // User declined
                                    eprintln!("Commit aborted by user.");
                                    ProgramOutcome::Success(ConfirmationOutcome::Declined)
                                }
                                ConfirmationOutcome::Cancelled => {
                                    // User cancelled (e.g., Ctrl+C)
                                    eprintln!("Confirmation cancelled by user.");
                                    ProgramOutcome::Success(ConfirmationOutcome::Cancelled)
                                }
                            }
                        }
                        Err(e) => {
                            // Error occurred during confirmation
                            eprintln!("Confirmation error: {}", e);
                            ProgramOutcome::Success(ConfirmationOutcome::Cancelled)
                        }
                    }
                }
            }
            Ok(None) => {
                // No diagnostics to show
                return ProgramOutcome::NoConfirmationNeeded;
            }
            Err(e) => {
                // Error occurred during submodule checking
                eprintln!("Submodule check error: {}", e);
                return ProgramOutcome::CheckError;
            }
        }
    }
    
    // No confirmation needed
    ProgramOutcome::NoConfirmationNeeded
}
