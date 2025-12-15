use git2::{Config, Repository};
use log::debug;

#[derive(Default)]
pub struct HookConfig {
    pub strict: Option<bool>,
    pub confirm_staging: Option<bool>,
    pub confirm_not_staging: Option<bool>,
}

pub fn get_config() -> HookConfig {
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