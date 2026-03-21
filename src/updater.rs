use crate::package_manager::{PackageManager};
use crate::notification::Notifier;
use crate::package_list;
use crate::system::System;
use log::{info, warn};

pub fn run_update<P: PackageManager, N: Notifier, S: System>(
    pm: P,
    notifier: N,
    sys: S,
    config: &crate::config::Config,
) -> anyhow::Result<()> {

    if sys.is_metered_connection() && !config.run_on_metered_connection {
        info!("Metered connection detected and running is not permitted, skipping updates.");
        return Ok(());
    }

    let allow_list = package_list::load(&config.allow_list_path, &config.default_source)?;
    let block_list = package_list::load(&config.block_list_path, &config.default_source)?;

    info!("Listing available updates...");
    let upgrades: Vec<_> = pm.list_upgrades()
        .into_iter()
        .filter(|u| {
            let in_allow = allow_list.is_empty()
                || allow_list.iter().any(|e| e.id == u.from.id && e.source == u.from.source);
            let in_block = block_list.iter().any(|e| e.id == u.from.id && e.source == u.from.source);
            let unknown_version = config.skip_unknown_version && u.from.version == "Unknown";
            in_allow && !in_block && !unknown_version
        })
        .collect();
    if upgrades.is_empty() {
        notifier.info("Winget Update", "No updates available");
        info!("No updates found.");
        return Ok(());
    }

    notifier.info("Winget Update", &format!("{} updates available", upgrades.len()));

    info!("Running updates...");

    let mut failed: Vec<String> = vec![];
    for package_upgrade in &upgrades {
        info!("Updating {} to {}", package_upgrade.from.name, package_upgrade.to.version);
        notifier.info("Winget Update", &format!("Updating {} to {}", package_upgrade.from.name, package_upgrade.to.version));
        match pm.upgrade(&package_upgrade.from) {
            Ok(upgraded) => info!("Updated {}: {} -> {}", package_upgrade.from.name, package_upgrade.from.version, upgraded.version),
            Err(e) => {
                warn!("Failed to upgrade {}: {}", package_upgrade.from.id, e);
                failed.push(package_upgrade.from.id.clone());
            }
        }
    }

    if !failed.is_empty() {
        let failed_list = failed.join(", ");
        notifier.warn("Winget Update", &format!("Failed to update: {}", failed_list));
        warn!("Failed updates: {}", failed_list);
    } else {
        notifier.info("Winget Update", "All updates completed successfully");
    }

    Ok(())
}
