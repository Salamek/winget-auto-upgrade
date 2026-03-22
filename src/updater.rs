use crate::hook::{self, HookContext};
use crate::notification::Notifier;
use crate::package_list::{self, PackageEntry, Scope};
use crate::package_manager::{PackageManager, UpgradeOptions};
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
    let override_list = package_list::load(&config.override_list_path, &config.default_source)?;
    let is_system = sys.is_running_as_system();

    // Returns true if a list entry applies in the current execution context
    let scope_matches = |e: &PackageEntry| match e.scope {
        Scope::All => true,
        Scope::Machine => is_system,
        Scope::User => !is_system,
    };

    info!("Listing available updates...");
    let upgrades: Vec<_> = pm
        .list_upgrades()
        .into_iter()
        .filter(|u| {
            let in_allow = allow_list.is_empty()
                || allow_list
                    .iter()
                    .any(|e| scope_matches(e) && e.id == u.from.id && e.source == u.from.source);
            let in_block = block_list
                .iter()
                .any(|e| scope_matches(e) && e.id == u.from.id && e.source == u.from.source);
            let unknown_version = config.skip_unknown_version && u.from.version == "Unknown";
            in_allow && !in_block && !unknown_version
        })
        .collect();
    if upgrades.is_empty() {
        notifier.info("Winget Update", "No updates available");
        info!("No updates found.");
        return Ok(());
    }

    notifier.info(
        "Winget Update",
        &format!("{} update(s) available", upgrades.len()),
    );

    info!("Running updates...");

    // Map a package to its UpgradeOptions from the override list (first scoped match wins)
    let resolve_options = |id: &str, source: &str| -> UpgradeOptions {
        override_list
            .iter()
            .find(|e| scope_matches(e) && e.id == id && e.source == source)
            .map(|e| UpgradeOptions {
                custom_args: e.custom_args.clone(),
                override_args: e.override_args.clone(),
                force_architecture: e.force_architecture.clone(),
                force_locale: e.force_locale.clone(),
                ignore_security_hash: e.ignore_security_hash,
                skip_dependencies: e.skip_depedencies,
            })
            .unwrap_or_default()
    };

    let mut failed: Vec<String> = vec![];
    for package_upgrade in &upgrades {
        info!(
            "Updating {} to {}",
            package_upgrade.from.name, package_upgrade.to.version
        );
        notifier.info(
            "Winget Update",
            &format!(
                "Updating {} → {}",
                package_upgrade.from.name, package_upgrade.to.version
            ),
        );

        let scope = if is_system { "machine" } else { "user" };
        let ctx = HookContext {
            id: &package_upgrade.from.id,
            name: &package_upgrade.from.name,
            source: &package_upgrade.from.source,
            scope: scope,
            version: &package_upgrade.from.version,
            available_version: &package_upgrade.to.version,
        };

        if let Some(hook) = &config.pre_update_hook {
            if let Err(e) = hook::run(hook, &config.hook_args_template, &ctx) {
                warn!(
                    "Pre-update hook failed for {}: {}",
                    package_upgrade.from.id, e
                );
            }
        }

        let options = resolve_options(&package_upgrade.from.id, &package_upgrade.from.source);
        match pm.upgrade(&package_upgrade.from, &options) {
            Ok(upgraded) => {
                info!(
                    "Updated {}: {} -> {}",
                    package_upgrade.from.name, package_upgrade.from.version, upgraded.version
                );
                notifier.success(
                    "Winget Update",
                    &format!(
                        "Updated {}: {} → {}",
                        package_upgrade.from.name, package_upgrade.from.version, upgraded.version
                    ),
                );
                if let Some(hook) = &config.post_update_hook {
                    let post_ctx = HookContext {
                        version: &upgraded.version,
                        ..ctx
                    };
                    if let Err(e) = hook::run(hook, &config.hook_args_template, &post_ctx) {
                        warn!(
                            "Post-update hook failed for {}: {}",
                            package_upgrade.from.id, e
                        );
                    }
                }
            }
            Err(e) => {
                warn!("Failed to upgrade {}: {}", package_upgrade.from.id, e);
                notifier.error(
                    "Winget Update",
                    &format!("Failed to upgrade {}: {}", package_upgrade.from.id, e),
                );
                failed.push(package_upgrade.from.id.clone());
            }
        }
    }

    if !failed.is_empty() {
        let failed_list = failed.join(", ");
        notifier.error(
            "Winget Update",
            &format!("Failed to update: {}", failed_list),
        );
        warn!("Failed updates: {}", failed_list);
    } else {
        notifier.success("Winget Update", "All updates completed successfully");
    }

    // After a SYSTEM run, trigger the user-context task if someone is logged in
    // so that Administrator-installed packages are also upgraded in user context.
    if is_system && sys.has_active_user_session() {
        info!("User session detected, triggering user-context upgrade task...");
        let status = std::process::Command::new("schtasks")
            .args(["/Run", "/TN", "winget-auto-upgrade-user"])
            .status();
        match status {
            Ok(s) if s.success() => info!("User-context task started."),
            Ok(s) => warn!("schtasks /Run exited with status {}", s),
            Err(e) => warn!("Failed to start user-context task: {}", e),
        }
    }

    Ok(())
}
