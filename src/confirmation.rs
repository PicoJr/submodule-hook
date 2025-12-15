use std::process::Termination;
use console::style;
use dialoguer::Confirm;
use dialoguer::theme::ColorfulTheme;
use crate::check_submodules::SubmodulesDiagnostic;

/// Enum representing the outcome of user confirmation
#[derive(Debug, PartialEq)]
pub enum ConfirmationOutcome {
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

pub fn ask_confirmation(diagnostics: &SubmodulesDiagnostic) -> anyhow::Result<ConfirmationOutcome> {
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