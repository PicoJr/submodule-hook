use clap::Parser;
use console::style;
use dialoguer::Confirm;
use dialoguer::theme::ColorfulTheme;
use git2::{Repository, SubmoduleIgnore};
use log::{debug, error, warn};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Make failed checks hard errors
    #[arg(long, default_value = "false")]
    strict: bool,
    /// Ask confirmation if commit contains a submodule update
    #[arg(long, default_value = "false")]
    careful: bool,
}

fn check_submodules(strict: bool, careful: bool) -> anyhow::Result<()> {
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

            let mut confirmation_message_lines = vec![];
            if !modified_not_staged_submodules.is_empty() {
                confirmation_message_lines.push(format!(
                    "{} {} {}",
                    style("The following submodules are").bold(),
                    style("modified but not staged").bold().red(),
                    style("for commit:").bold(),
                ));
                for name in &modified_not_staged_submodules {
                    confirmation_message_lines.push(format!(
                        "* {} (`git add {name}` to add submodule to staging)",
                        style(name).bold().red(),
                    ));
                }
            }
            if !modified_staged_submodules.is_empty() {
                confirmation_message_lines.push(format!(
                    "{} {} {}",
                    style("The following submodules are").bold(),
                    style("modified and staged").bold().green(),
                    style("for commit:").bold(),
                ));
                for name in &modified_staged_submodules {
                    confirmation_message_lines.push(format!(
                        "* `{}` (`git restore --staged {name}` to remove submodule from staging)",
                        style(name).bold().green(),
                    ));
                }
            }
            confirmation_message_lines.push("Do you wish to continue anyway?".to_string());
            let ask_confirmation = !modified_not_staged_submodules.is_empty()
                || (!modified_staged_submodules.is_empty() && careful);

            if ask_confirmation {
                let confirmation = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt(confirmation_message_lines.join("\n"))
                    .default(false)
                    .show_default(true)
                    .interact()?;

                if !confirmation {
                    anyhow::bail!("Commit aborted by user.")
                }
            }
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
    Ok(())
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();
    check_submodules(args.strict, args.careful)
}
