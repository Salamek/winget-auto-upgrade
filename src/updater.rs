use crate::package_manager::{PackageManager, UpdateResult};
use crate::notification::Notifier;
use log::{info, warn};

pub fn run_update<P: PackageManager, N: Notifier>(
    pm: P,
    notifier: N,
    _config: &crate::config::Config,
) -> anyhow::Result<()> {
    info!("Listing available updates...");
    let upgrades = pm.list_upgrades();
    dbg!(&upgrades);
    if upgrades.is_empty() {
        notifier.info("Winget Update", "No updates available");
        info!("No updates found.");
        return Ok(());
    }

    notifier.info("Winget Update", &format!("{} updates available", upgrades.len()));

    info!("Running updates...");
    let result: UpdateResult = pm.upgrade_all()?;

    for pkg in &result.updated {
        info!("Updated {}: {} -> {}", pkg.from.name, pkg.from.version, pkg.to.version);
    }

    if !result.failed.is_empty() {
        let failed_list = result.failed.join(", ");
        notifier.warn("Winget Update", &format!("Failed to update: {}", failed_list));
        warn!("Failed updates: {}", failed_list);
    } else {
        notifier.info("Winget Update", "All updates completed successfully");
    }

    Ok(())
}
