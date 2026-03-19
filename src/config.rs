use toml;

use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub log_path: String,
    pub allow_list: Vec<String>,
    pub block_list: Vec<String>,
    // Add other settings as needed
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_path: "winget-update.log".to_string(),
            allow_list: vec![],
            block_list: vec![],
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
