use clap::Parser;
use console::style;
use dialoguer::Confirm;
use dialoguer::theme::ColorfulTheme;
use git2::{Config, Repository, SubmoduleIgnore};
use log::{debug, error, warn};

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

struct SubmodulesDiagnostic {
    modified_not_staged_submodules: Vec<String>,
    modified_staged_submodules: Vec<String>,
}

fn check_submodules(strict: bool) -> anyhow::Result<Option<SubmodulesDiagnostic>> {
    if let Ok(repo) = Repository::open(".") {
        if let Ok(submodules) = repo.submodules() {
            let mut modified_not_staged_submodules: Vec<String> = vec![];
            let mut modified_staged_submodules: Vec<String> = vec![];
            for submodule in submodules {
                if let Some(name) = submodule.name() {
                    debug!("checking submodule: {name}");
                    let status = repo.submodule_status(name, SubmoduleIgnore::None)?;
                    if status.is_wd_modified() {
                        debug!("{name} is modified but not staged");
                        modified_not_staged_submodules.push(String::from(name));
                    }
                    if status.is_index_modified() {
                        debug!("{name} is modified and staged");
                        modified_staged_submodules.push(String::from(name));
                    }
                } else {
                    warn!("submodule does not have a name");
                }
            }
            return Ok(Some(SubmodulesDiagnostic {
                modified_not_staged_submodules,
                modified_staged_submodules,
            }));
        } else {
            error!("failed to list submodules");
            if strict {
                anyhow::bail!("Failed to list submodules.");
            }
        }
    } else {
        error!("failed to open git repository");
        if strict {
            anyhow::bail!("Unable to open repository");
        }
    }
    Ok(None)
}

fn ask_confirmation(
    diagnostics: &SubmodulesDiagnostic,
) -> anyhow::Result<()> {
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
    confirmation_message_lines.push("Do you wish to continue anyway?".to_string());

    let confirmation = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(confirmation_message_lines.join("\n"))
        .default(false)
        .show_default(true)
        .report(false)
        .interact()?;

    if !confirmation {
        anyhow::bail!("Commit aborted by user.")
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
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
        let diagnostics = check_submodules(strict)?;
        if let Some(diagnostics) = diagnostics {
            let prompt_for_confirmation = (!diagnostics.modified_not_staged_submodules.is_empty()
                && confirm_not_staging)
                || (!diagnostics.modified_staged_submodules.is_empty() && confirm_staging);
            if prompt_for_confirmation {
                ask_confirmation(&diagnostics)?;
            }
        }
    }
    Ok(())
}
