mod config;
mod package_list;
mod logging;
mod updater;
mod notification;
mod package_manager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::load_config("config.toml")?;
    logging::init(&config)?;

    #[cfg(target_os = "windows")]
    let notifier = notification::WindowsNotifier::new(config.notification_level.clone());

    #[cfg(not(target_os = "windows"))]
    let notifier = notification::StubNotifier::new(config.notification_level.clone());

    let pm = package_manager::Winget::new();

    updater::run_update(pm, notifier, &config)?;
    Ok(())
}
