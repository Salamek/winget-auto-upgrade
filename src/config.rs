use toml;

use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Default, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum NotificationLevel {
    #[default]
    All,
    Success,
    Error,
    None,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub log_path: String,
    pub default_source: String,
    pub allow_list_path: String,
    pub block_list_path: String,
    pub skip_unknown_version: bool,
    pub run_on_metered_connection: bool,
    pub notification_level: NotificationLevel,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_path: "winget-update.log".to_string(),
            default_source: "winget".to_string(),
            allow_list_path: "file:///home/sadam/projects/winget-autoupgrade/target/debug/allow_list.toml".to_string(),
            block_list_path: "file:///home/sadam/projects/winget-autoupgrade/target/debug/block_list.toml".to_string(),
            skip_unknown_version: true,
            run_on_metered_connection: false,
            notification_level: NotificationLevel::default()
        }
    }
}

pub fn load_config(path: &str) -> anyhow::Result<Config> {
    let content = fs::read_to_string(path).unwrap_or_else(|_| String::new());
    if content.trim().is_empty() {
        Ok(Config::default())
    } else {
        let cfg: Config = toml::from_str(&content)?;
        Ok(cfg)
    }
}
