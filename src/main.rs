mod config;
mod hook;
mod logging;
mod notification;
mod package_list;
mod package_manager;
mod system;
mod updater;

use clap::{Parser, Subcommand};
use system::System;

#[derive(Parser)]
#[command(name = "winget-autoupgrade", about = "Automatic winget package updater")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the automatic update process (default when no subcommand is given)
    Update,
    /// Show a toast notification in the current user session (used internally by the updater)
    #[command(hide = true)]
    Notify {
        #[arg(long)]
        title: String,
        #[arg(long)]
        message: String,
        /// Icon name: info | success | warning | error
        #[arg(long)]
        icon: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Update) {
        Commands::Update => run_update(),
        Commands::Notify { title, message, icon } => run_notify(&title, &message, &icon),
    }
}

fn run_update() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::load_config("config.toml")?;
    logging::init(&config)?;

    #[cfg(target_os = "windows")]
    let sys = system::WindowsSystem::new();
    #[cfg(not(target_os = "windows"))]
    let sys = system::StubSystem::new();

    #[cfg(target_os = "windows")]
    let notifier = notification::WindowsNotifier::new(config.notification_level.clone(), sys.is_running_as_system());
    #[cfg(not(target_os = "windows"))]
    let notifier = notification::StubNotifier::new(config.notification_level.clone());

    let pm = package_manager::Winget::new();
    updater::run_update(pm, notifier, sys, &config)?;
    Ok(())
}

fn run_notify(title: &str, message: &str, icon: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        notification::WindowsNotifier::show_for_notify(title, message, icon);
        // Keep the process alive briefly so the WinRT toast system has time to register
        // the notification before we exit — same approach as WAU's Start-Sleep 3.
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
    #[cfg(not(target_os = "windows"))]
    println!("[NOTIFY] {title}: {message} (icon: {icon})");
    Ok(())
}
