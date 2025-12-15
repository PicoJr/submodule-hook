use check_submodules::SubmodulesDiagnostic;
use clap::Parser;
use console::style;
use dialoguer::Confirm;
use dialoguer::theme::ColorfulTheme;
use git2::{Config, Repository};
use log::debug;
use std::path::PathBuf;
use std::process::Termination;

mod check_submodules;

/// Enum representing the outcome of user confirmation
#[derive(Debug, PartialEq)]
enum ConfirmationOutcome {
    /// User confirmed the action
    Confirmed,
    /// User declined the action
    Declined,
    /// User cancelled/interrupted the confirmation (e.g., Ctrl+C)
    Cancelled,
}

impl Termination for ConfirmationOutcome {
    fn report(self) -> std::process::ExitCode {
        match self {
            ConfirmationOutcome::Confirmed => std::process::ExitCode::SUCCESS,
            ConfirmationOutcome::Declined => std::process::ExitCode::from(1),
            ConfirmationOutcome::Cancelled => std::process::ExitCode::from(2),
        }
    }
}

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

#[derive(Default)]
struct HookConfig {
    strict: Option<bool>,
    confirm_staging: Option<bool>,
    confirm_not_staging: Option<bool>,
}

fn get_config() -> HookConfig {
    let mut config = HookConfig::default();
    let config_name = "submodulehook".to_string();
    let strict_option = format!("{config_name}.strict");
    let confirm_staging_option = format!("{config_name}.staging");
    let confirm_not_staging_option = format!("{config_name}.notstaging");

    // 0 try reading from global config
    if let Ok(global_config) = Config::open_default() {
        if let Ok(value) = global_config.get_string(strict_option.as_str()) {
            debug!("found global config: {strict_option} = {value}");
            config.strict = Some(value == "true");
        }
        if let Ok(value) = global_config.get_string(confirm_staging_option.as_str()) {
            debug!("found global config: {confirm_staging_option} = {value}");
            config.confirm_staging = Some(value == "true");
        }
        if let Ok(value) = global_config.get_string(confirm_not_staging_option.as_str()) {
            debug!("found global config: {confirm_not_staging_option} = {value}");
            config.confirm_not_staging = Some(value == "true");
        }
    }

    // 1 try reading from local config
    if let Ok(repo) = Repository::open(".") {
        if let Ok(local_config) = repo.config() {
            if let Ok(value) = local_config.get_string(strict_option.as_str()) {
                debug!("found local config: {strict_option} = {value}");
                config.strict = Some(value == "true");
            }
            if let Ok(value) = local_config.get_string(confirm_staging_option.as_str()) {
                debug!("found local config: {confirm_staging_option} = {value}");
                config.confirm_staging = Some(value == "true");
            }
            if let Ok(value) = local_config.get_string(confirm_not_staging_option.as_str()) {
                debug!("found local config: {confirm_not_staging_option} = {value}");
                config.confirm_not_staging = Some(value == "true");
            }
        }
    }
    config
}

fn ask_confirmation(diagnostics: &SubmodulesDiagnostic) -> anyhow::Result<ConfirmationOutcome> {
    let mut confirmation_message_lines = vec![];
    if !diagnostics.modified_not_staged_submodules.is_empty() {
        confirmation_message_lines.push(format!(
            "{} {} {}",
            style("The following submodules are").bold(),
            style("modified but not staged").bold().red(),
            style("for commit:").bold(),
        ));
        for name in &diagnostics.modified_not_staged_submodules {
            confirmation_message_lines.push(format!(
                "* {} (`git add {name}` to add submodule to staging)",
                style(name).bold().red(),
            ));
        }
    }
    if !diagnostics.modified_staged_submodules.is_empty() {
        confirmation_message_lines.push(format!(
            "{} {} {}",
            style("The following submodules are").bold(),
            style("modified and staged").bold().green(),
            style("for commit:").bold(),
        ));
        for name in &diagnostics.modified_staged_submodules {
            confirmation_message_lines.push(format!(
                "* {} (`git restore --staged {name}` to remove submodule from staging)",
                style(name).bold().green(),
            ));
        }
    }

    println!("{}", confirmation_message_lines.join("\n"));
    match Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you wish to continue anyway?".to_string())
        .default(false)
        .show_default(true)
        .report(true)
        .interact() {
        Ok(confirmation) => {
            if confirmation {
                Ok(ConfirmationOutcome::Confirmed)
            } else {
                Ok(ConfirmationOutcome::Declined)
            }
        }
        Err(_) => Ok(ConfirmationOutcome::Cancelled),
    }
}

fn main() -> ProgramOutcome {
    env_logger::init();
    let args = Args::parse();
    let cli_config = HookConfig {
        strict: args.strict,
        confirm_staging: args.confirm_staging,
        confirm_not_staging: args.confirm_not_staging,
    };
    let git_config = get_config();
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
                    match ask_confirmation(&diagnostics) {
                        Ok(outcome) => {
                            match outcome {
                                ConfirmationOutcome::Confirmed => {
                                    // User confirmed
                                    return ProgramOutcome::Success(ConfirmationOutcome::Confirmed);
                                }
                                ConfirmationOutcome::Declined => {
                                    // User declined
                                    eprintln!("Commit aborted by user.");
                                    return ProgramOutcome::Success(ConfirmationOutcome::Declined);
                                }
                                ConfirmationOutcome::Cancelled => {
                                    // User cancelled (e.g., Ctrl+C)
                                    eprintln!("Confirmation cancelled by user.");
                                    return ProgramOutcome::Success(ConfirmationOutcome::Cancelled);
                                }
                            }
                        }
                        Err(e) => {
                            // Error occurred during confirmation
                            eprintln!("Confirmation error: {}", e);
                            return ProgramOutcome::Success(ConfirmationOutcome::Cancelled);
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
