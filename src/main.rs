use clap::Parser;
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
}

fn check_submodules(strict: bool) -> anyhow::Result<()> {
    if let Ok(repo) = Repository::open(".") {
        if let Ok(submodules) = repo.submodules() {
            let mut overlooked_submodules: Vec<String> = vec![];
            for submodule in submodules {
                if let Some(name) = submodule.name() {
                    debug!("checking submodule: {name}");
                    let status = repo.submodule_status(name, SubmoduleIgnore::None)?;
                    if status.is_wd_modified() {
                        debug!("{name} is modified but not staged");
                        overlooked_submodules.push(String::from(name));
                    }
                } else {
                    warn!("submodule does not have a name");
                }
            }

            if !overlooked_submodules.is_empty() {
                let mut confirmation_message_lines = vec![
                    "The following submodules are modified but not staged for commit:".to_string(),
                ];
                for name in overlooked_submodules {
                    confirmation_message_lines.push(format!("* `{name}` is modified and not staged, (`git add {name}` to add submodule to staging)"));
                }
                confirmation_message_lines.push("Do you wish to continue anyway?".to_string());

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
    check_submodules(args.strict)
}
