use git2::{Repository, SubmoduleIgnore};
use log::{debug, error, warn};
use std::path::Path;

#[derive(Debug)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{IndexAddOption, Oid, Signature};
    use std::fs;

    use tempfile::TempDir;

    // ========== Test Helpers ==========

    /// Creates a temporary git repository with an initial commit
    /// Returns (TempDir, Repository) - TempDir must be kept alive for the repository to remain valid
    fn create_temp_repo() -> anyhow::Result<(TempDir, Repository)> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_path_buf();
        let repo = Repository::init(&temp_path)?;

        // Create initial commit (required for submodule operations)
        let sig = Signature::now("Test User", "test@example.com")?;
        let tree_id = {
            let mut index = repo.index()?;
            // Create a README file
            let readme_path = temp_path.join("README.md");
            fs::write(&readme_path, "# Test Repository\n")?;
            index.add_path(std::path::Path::new("README.md"))?;
            index.write()?;
            index.write_tree()?
        };
        let tree = repo.find_tree(tree_id)?;
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;
        drop(tree); // Drop tree to release the borrow on repo

        Ok((temp_dir, repo))
    }

    /// Creates a commit in the given repository with a new file
    fn create_commit(repo: &Repository, message: &str) -> anyhow::Result<Oid> {
        let sig = Signature::now("Test User", "test@example.com")?;

        // Create a unique file for this commit
        let file_name = format!("file_{}.txt", message.replace(' ', "_"));
        let file_path = repo.workdir().unwrap().join(&file_name);
        fs::write(&file_path, format!("Content for {}", message))?;

        // Stage the file
        let mut index = repo.index()?;
        index.add_path(std::path::Path::new(&file_name))?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        // Get parent commit
        let parent_commit = repo.head()?.peel_to_commit()?;

        // Create commit
        let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent_commit])?;

        Ok(oid)
    }

    /// Adds a submodule to the parent repository
    /// Returns (TempDir, submodule_name) - TempDir must be kept alive
    fn add_submodule(
        parent_repo: &Repository,
        submodule_name: &str,
    ) -> anyhow::Result<(TempDir, String)> {
        // Create a separate repository for the submodule
        let (submodule_temp_dir, submodule_repo) = create_temp_repo()?;
        let submodule_path = submodule_temp_dir.path().to_path_buf();
        let submodule_url = format!("file://{}", submodule_path.display());

        // Add submodule to parent repository
        let mut submodule =
            parent_repo.submodule(&submodule_url, std::path::Path::new(submodule_name), false)?;

        // Clone the submodule repository
        let _cloned_repo = submodule.clone(None)?;

        // Finalize the submodule addition
        submodule.add_finalize()?;

        // Commit the submodule addition
        let sig = Signature::now("Test User", "test@example.com")?;
        let mut index = parent_repo.index()?;
        index.add_all(
            [".gitmodules", submodule_name].iter(),
            IndexAddOption::DEFAULT,
            None,
        )?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = parent_repo.find_tree(tree_id)?;
        let parent_commit = parent_repo.head()?.peel_to_commit()?;
        parent_repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &format!("Add submodule {}", submodule_name),
            &tree,
            &[&parent_commit],
        )?;
        drop(tree); // Drop tree to release the borrow

        // Keep the submodule repo alive by storing it
        drop(submodule_repo);

        Ok((submodule_temp_dir, submodule_name.to_string()))
    }

    /// Modifies the submodule's working directory by creating a commit
    /// This makes the submodule modified but NOT staged in the parent
    fn modify_submodule_wd(parent_repo: &Repository, submodule_name: &str) -> anyhow::Result<()> {
        let submodule_path = parent_repo.workdir().unwrap().join(submodule_name);
        let submodule_repo = Repository::open(&submodule_path)?;

        // Create a new commit in the submodule
        create_commit(&submodule_repo, "Submodule modification")?;

        Ok(())
    }

    /// Stages the submodule changes in the parent repository's index
    fn stage_submodule(parent_repo: &Repository, submodule_name: &str) -> anyhow::Result<()> {
        let mut index = parent_repo.index()?;
        index.add_path(std::path::Path::new(submodule_name))?;
        index.write()?;
        Ok(())
    }

    // ========== Tests for Helpers ==========

    #[test]
    fn test_create_temp_repo() {
        let result = create_temp_repo();
        assert!(result.is_ok());

        let (_temp_dir, repo) = result.unwrap();

        // Verify repository is valid
        assert!(!repo.is_bare());
        assert!(repo.head().is_ok());

        // Verify initial commit exists
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        assert_eq!(commit.message().unwrap(), "Initial commit");
    }

    #[test]
    fn test_create_commit() {
        let (_temp_dir, repo) = create_temp_repo().unwrap();

        let result = create_commit(&repo, "Test commit");
        assert!(result.is_ok());

        // Verify commit was created
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(commit.message().unwrap(), "Test commit");
    }

    #[test]
    fn test_add_submodule() {
        let (_parent_temp_dir, parent_repo) = create_temp_repo().unwrap();

        let result = add_submodule(&parent_repo, "test-submodule");
        assert!(result.is_ok());

        let (_submodule_temp_dir, submodule_name) = result.unwrap();

        // Verify submodule was added
        let submodules = parent_repo.submodules().unwrap();
        assert_eq!(submodules.len(), 1);
        assert_eq!(submodules[0].name().unwrap(), submodule_name);

        // Verify submodule directory exists
        let submodule_path = parent_repo.workdir().unwrap().join(&submodule_name);
        assert!(submodule_path.exists());
    }

    #[test]
    fn test_modify_submodule_wd() {
        let (_parent_temp_dir, parent_repo) = create_temp_repo().unwrap();
        let (_submodule_temp_dir, submodule_name) =
            add_submodule(&parent_repo, "test-submodule").unwrap();

        let result = modify_submodule_wd(&parent_repo, &submodule_name);
        assert!(result.is_ok());

        // Verify submodule status shows modification
        let status = parent_repo
            .submodule_status(&submodule_name, SubmoduleIgnore::None)
            .unwrap();
        assert!(status.is_wd_modified());
    }

    #[test]
    fn test_stage_submodule() {
        let (_parent_temp_dir, parent_repo) = create_temp_repo().unwrap();
        let (_submodule_temp_dir, submodule_name) =
            add_submodule(&parent_repo, "test-submodule").unwrap();

        // Modify submodule first
        modify_submodule_wd(&parent_repo, &submodule_name).unwrap();

        // Stage the submodule
        let result = stage_submodule(&parent_repo, &submodule_name);
        assert!(result.is_ok());

        // Verify submodule is staged
        let status = parent_repo
            .submodule_status(&submodule_name, SubmoduleIgnore::None)
            .unwrap();
        assert!(status.is_index_modified());
    }

    // ========== Tests for check_submodules ==========

    #[test]
    fn test_no_submodules() {
        let (_temp_dir, repo) = create_temp_repo().unwrap();
        let repo_path = repo.workdir().unwrap();

        let result = check_submodules(false, repo_path);
        assert!(result.is_ok());

        let diagnostic = result.unwrap();
        assert!(diagnostic.is_some());

        let diagnostic = diagnostic.unwrap();
        assert!(diagnostic.modified_not_staged_submodules.is_empty());
        assert!(diagnostic.modified_staged_submodules.is_empty());
    }

    #[test]
    fn test_unmodified_submodules() {
        let (_parent_temp_dir, parent_repo) = create_temp_repo().unwrap();
        let (_submodule_temp_dir, _submodule_name) =
            add_submodule(&parent_repo, "clean-submodule").unwrap();

        let repo_path = parent_repo.workdir().unwrap();
        let result = check_submodules(false, repo_path);
        assert!(result.is_ok());

        let diagnostic = result.unwrap().unwrap();
        assert!(diagnostic.modified_not_staged_submodules.is_empty());
        assert!(diagnostic.modified_staged_submodules.is_empty());
    }

    #[test]
    fn test_modified_not_staged_submodule() {
        let (_parent_temp_dir, parent_repo) = create_temp_repo().unwrap();
        let (_submodule_temp_dir, submodule_name) =
            add_submodule(&parent_repo, "modified-submodule").unwrap();

        // Modify submodule but don't stage
        modify_submodule_wd(&parent_repo, &submodule_name).unwrap();

        let repo_path = parent_repo.workdir().unwrap();
        let result = check_submodules(false, repo_path);
        assert!(result.is_ok());

        let diagnostic = result.unwrap().unwrap();
        assert_eq!(diagnostic.modified_not_staged_submodules.len(), 1);
        assert_eq!(diagnostic.modified_not_staged_submodules[0], submodule_name);
        assert!(diagnostic.modified_staged_submodules.is_empty());
    }

    #[test]
    fn test_modified_staged_submodule() {
        let (_parent_temp_dir, parent_repo) = create_temp_repo().unwrap();
        let (_submodule_temp_dir, submodule_name) =
            add_submodule(&parent_repo, "staged-submodule").unwrap();

        // Modify and stage submodule
        modify_submodule_wd(&parent_repo, &submodule_name).unwrap();
        stage_submodule(&parent_repo, &submodule_name).unwrap();

        let repo_path = parent_repo.workdir().unwrap();
        let result = check_submodules(false, repo_path);
        assert!(result.is_ok());

        let diagnostic = result.unwrap().unwrap();
        assert!(diagnostic.modified_not_staged_submodules.is_empty());
        assert_eq!(diagnostic.modified_staged_submodules.len(), 1);
        assert_eq!(diagnostic.modified_staged_submodules[0], submodule_name);
    }

    #[test]
    fn test_both_modified_submodules() {
        let (_parent_temp_dir, parent_repo) = create_temp_repo().unwrap();

        // Add first submodule - modified but not staged
        let (_submodule1_temp_dir, submodule1_name) =
            add_submodule(&parent_repo, "submodule1").unwrap();
        modify_submodule_wd(&parent_repo, &submodule1_name).unwrap();

        // Add second submodule - modified and staged
        let (_submodule2_temp_dir, submodule2_name) =
            add_submodule(&parent_repo, "submodule2").unwrap();
        modify_submodule_wd(&parent_repo, &submodule2_name).unwrap();
        stage_submodule(&parent_repo, &submodule2_name).unwrap();

        let repo_path = parent_repo.workdir().unwrap();
        let result = check_submodules(false, repo_path);
        assert!(result.is_ok());

        let diagnostic = result.unwrap().unwrap();
        assert_eq!(diagnostic.modified_not_staged_submodules.len(), 1);
        assert_eq!(
            diagnostic.modified_not_staged_submodules[0],
            submodule1_name
        );
        assert_eq!(diagnostic.modified_staged_submodules.len(), 1);
        assert_eq!(diagnostic.modified_staged_submodules[0], submodule2_name);
    }

    #[test]
    fn test_strict_mode_invalid_repo() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_path = temp_dir.path();

        // This should fail in strict mode
        let result = check_submodules(true, invalid_path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unable to open repository")
        );
    }

    #[test]
    fn test_non_strict_mode_invalid_repo() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_path = temp_dir.path();

        // This should return Ok(None) in non-strict mode
        let result = check_submodules(false, invalid_path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
