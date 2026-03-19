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

#[derive(Debug)]
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
            allow_list_path: "file://allow_list.toml".to_string(),
            block_list_path: "file://block_list.toml".to_string(),
            skip_unknown_version: true,
            run_on_metered_connection: false,
            notification_level: NotificationLevel::default(),
        }
    }
}

#[derive(Deserialize, Default)]
struct RawConfig {
    log_path: Option<String>,
    default_source: Option<String>,
    allow_list_path: Option<String>,
    block_list_path: Option<String>,
    skip_unknown_version: Option<bool>,
    run_on_metered_connection: Option<bool>,
    notification_level: Option<NotificationLevel>,
}

pub fn load_config(path: &str) -> anyhow::Result<Config> {
    let content = fs::read_to_string(path).unwrap_or_default();
    let raw: RawConfig = if content.trim().is_empty() {
        RawConfig::default()
    } else {
        toml::from_str(&content)?
    };
    let defaults = Config::default();
    Ok(Config {
        log_path:                raw.log_path.unwrap_or(defaults.log_path),
        default_source:          raw.default_source.unwrap_or(defaults.default_source),
        allow_list_path:         raw.allow_list_path.unwrap_or(defaults.allow_list_path),
        block_list_path:         raw.block_list_path.unwrap_or(defaults.block_list_path),
        skip_unknown_version:    raw.skip_unknown_version.unwrap_or(defaults.skip_unknown_version),
        run_on_metered_connection: raw.run_on_metered_connection.unwrap_or(defaults.run_on_metered_connection),
        notification_level:      raw.notification_level.unwrap_or(defaults.notification_level),
    })
}
