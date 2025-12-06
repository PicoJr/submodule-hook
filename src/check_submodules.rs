use git2::{Repository, SubmoduleIgnore};
use log::{debug, error, warn};
use std::path::Path;

pub struct SubmodulesDiagnostic {
    pub modified_not_staged_submodules: Vec<String>,
    pub modified_staged_submodules: Vec<String>,
}
pub fn check_submodules(strict: bool, path: &Path) -> anyhow::Result<Option<SubmodulesDiagnostic>> {
    if let Ok(repo) = Repository::open(path) {
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
