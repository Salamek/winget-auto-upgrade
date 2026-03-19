mod config;
mod logging;
mod updater;
mod notification;
mod package_manager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::load_config("config.toml")?;
    logging::init(&config)?;

    #[cfg(target_os = "windows")]
    let notifier = notification::WindowsNotifier::new();

    #[cfg(not(target_os = "windows"))]
    let notifier = notification::StubNotifier::new();

    let pm = package_manager::Winget::new();

    updater::run_update(pm, notifier, &config)?;
    Ok(())
}
