use winget_autoupgeade::package_manager::{StubWinget, PackageManager};
use winget_autoupgeade::notification::{StubNotifier, Notifier};
use winget_autoupgeade::updater;
use crate::config::Config;

#[test]
fn test_stub_winget_update() {
    let pm = StubWinget::new();
    let notifier = StubNotifier::new();
    let config = Config::default();

    // Run the updater using the stub
    let result = updater::run_update(pm, notifier, &config);

    // Ensure it completed successfully
    assert!(result.is_ok());

    // Additionally check that stub returns expected updates
    let upgrades = StubWinget::new().list_upgrades();
    assert_eq!(upgrades.len(), 2); // matches stub definition
    assert_eq!(upgrades[0].name, "Foo");
    assert_eq!(upgrades[1].name, "Bar");
}
