use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum NotificationLevel {
    #[default]
    All,
    Success,
    Error,
    None,
}

impl NotificationLevel {
    #[cfg(target_os = "windows")]
    fn from_wau_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "Full"     => Some(Self::All),
            "Success only" => Some(Self::Success),
            "Errors only"   => Some(Self::Error),
            "None"    => Some(Self::None),
            _         => Option::None,
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub log_path: Option<String>,
    pub default_source: String,
    pub allow_list_path: String,
    pub block_list_path: String,
    pub override_list_path: String,
    pub pre_update_hook: Option<PathBuf>,
    pub post_update_hook: Option<PathBuf>,
    pub hook_args_template: String,
    pub skip_unknown_version: bool,
    pub run_on_metered_connection: bool,
    pub notification_level: NotificationLevel,
    pub max_log_files: u32,
    pub max_log_size: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_path: None,
            default_source: "winget".to_string(),
            allow_list_path: "file://allow_list.toml".to_string(),
            block_list_path: "file://block_list.toml".to_string(),
            override_list_path: "file://override_list.toml".to_string(),
            pre_update_hook: None,
            post_update_hook: None,
            hook_args_template: "{id} {source} {version} {available_version}".to_string(),
            skip_unknown_version: true,
            run_on_metered_connection: false,
            notification_level: NotificationLevel::default(),
            max_log_files: 3,
            max_log_size: 1_048_576, // 1 MB
        }
    }
}

#[derive(Deserialize, Default)]
struct RawConfig {
    log_path: Option<String>,
    default_source: Option<String>,
    allow_list_path: Option<String>,
    block_list_path: Option<String>,
    override_list_path: Option<String>,
    pre_update_hook: Option<PathBuf>,
    post_update_hook: Option<PathBuf>,
    hook_args_template: Option<String>,
    skip_unknown_version: Option<bool>,
    run_on_metered_connection: Option<bool>,
    notification_level: Option<NotificationLevel>,
    max_log_files: Option<u32>,
    max_log_size: Option<u64>,
}

impl RawConfig {
    // Apply other on top of self, Some values in other win, None values keep self
    #[cfg(target_os = "windows")]
    fn override_with(self, other: RawConfig) -> RawConfig {
        RawConfig {
            log_path:                 other.log_path.or(self.log_path),
            default_source:           other.default_source.or(self.default_source),
            allow_list_path:          other.allow_list_path.or(self.allow_list_path),
            block_list_path:          other.block_list_path.or(self.block_list_path),
            override_list_path:       other.override_list_path.or(self.override_list_path),
            pre_update_hook:          other.pre_update_hook.or(self.pre_update_hook),
            post_update_hook:         other.post_update_hook.or(self.post_update_hook),
            hook_args_template:       other.hook_args_template.or(self.hook_args_template),
            skip_unknown_version:     other.skip_unknown_version.or(self.skip_unknown_version),
            run_on_metered_connection: other.run_on_metered_connection.or(self.run_on_metered_connection),
            notification_level:       other.notification_level.or(self.notification_level),
            max_log_files:            other.max_log_files.or(self.max_log_files),
            max_log_size:             other.max_log_size.or(self.max_log_size),
        }
    }

    fn into_config(self) -> Config {
        let defaults = Config::default();
        Config {
            log_path:                 self.log_path.or(defaults.log_path),
            default_source:           self.default_source.unwrap_or(defaults.default_source),
            allow_list_path:          self.allow_list_path.unwrap_or(defaults.allow_list_path),
            block_list_path:          self.block_list_path.unwrap_or(defaults.block_list_path),
            override_list_path:       self.override_list_path.unwrap_or(defaults.override_list_path),
            pre_update_hook:          self.pre_update_hook.or(defaults.pre_update_hook),
            post_update_hook:         self.post_update_hook.or(defaults.post_update_hook),
            hook_args_template:       self.hook_args_template.unwrap_or(defaults.hook_args_template),
            skip_unknown_version:     self.skip_unknown_version.unwrap_or(defaults.skip_unknown_version),
            run_on_metered_connection: self.run_on_metered_connection.unwrap_or(defaults.run_on_metered_connection),
            notification_level:       self.notification_level.unwrap_or(defaults.notification_level),
            max_log_files:            self.max_log_files.unwrap_or(defaults.max_log_files),
            max_log_size:             self.max_log_size.unwrap_or(defaults.max_log_size),
        }
    }
}

#[cfg(target_os = "windows")]
fn load_wau_registry_layer() -> RawConfig {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let key_path = "SOFTWARE\\Romanitho\\Winget-AutoUpdate";

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = match hklm.open_subkey(key_path) {
        Ok(k) => k,
        Err(_) => return RawConfig::default(),
    };

    let get_string = |name: &str| -> Option<String> { key.get_value(name).ok() };
    let get_bool   = |name: &str| -> Option<bool>   { key.get_value::<u32, _>(name).ok().map(|v| v != 0) };

    RawConfig {
        default_source:           get_string("WAU_WingetSourceCustom"),
        run_on_metered_connection: get_bool("WAU_DoNotRunOnMetered").map(|v| !v),
        notification_level:       get_string("WAU_NotificationLevel")
                                    .and_then(|s| NotificationLevel::from_wau_str(&s)),
        max_log_files:            key.get_value::<u32, _>("WAU_MaxLogFiles").ok(),
        max_log_size:             key.get_value::<u32, _>("WAU_MaxLogSize").ok().map(|v| v as u64),
        ..RawConfig::default()
    }
}

#[cfg(target_os = "windows")]
fn load_wau_policy_registry_layer() -> RawConfig {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let key_path = "Software\\Policies\\Romanitho\\Winget-AutoUpdate";

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = match hklm.open_subkey(key_path) {
        Ok(k) => k,
        Err(_) => return RawConfig::default(),
    };

    let get_string = |name: &str| -> Option<String> { key.get_value(name).ok() };
    let get_bool   = |name: &str| -> Option<bool>   { key.get_value::<u32, _>(name).ok().map(|v| v != 0) };

    RawConfig {
        default_source:           get_string("WAU_WingetSourceCustom"),
        run_on_metered_connection: get_bool("WAU_DoNotRunOnMetered").map(|v| !v),
        notification_level:       get_string("WAU_NotificationLevel")
                                    .and_then(|s| NotificationLevel::from_wau_str(&s)),
        max_log_files:            key.get_value::<u32, _>("WAU_MaxLogFiles").ok(),
        max_log_size:             key.get_value::<u32, _>("WAU_MaxLogSize").ok().map(|v| v as u64),
        ..RawConfig::default()
    }
}

pub fn load_config(path: &str) -> anyhow::Result<Config> {
    // Layer 1: config.toml
    let content = fs::read_to_string(path).unwrap_or_default();
    let file_layer: RawConfig = if content.trim().is_empty() {
        RawConfig::default()
    } else {
        toml::from_str(&content)?
    };

    // Layer 2 -> 3: Windows registry (only on Windows)
    #[cfg(target_os = "windows")]
    let file_layer = {
        let wau     = load_wau_registry_layer();
        let policy  = load_wau_policy_registry_layer();
        file_layer.override_with(wau).override_with(policy)
    };

    Ok(file_layer.into_config())
}
