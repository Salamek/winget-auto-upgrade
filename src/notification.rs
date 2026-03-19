pub trait Notifier {
    fn info(&self, title: &str, message: &str);
    fn warn(&self, title: &str, message: &str);
}

#[cfg(not(target_os = "windows"))]
pub struct StubNotifier;

#[cfg(not(target_os = "windows"))]
impl StubNotifier {
    pub fn new() -> Self {
        StubNotifier
    }
}

#[cfg(not(target_os = "windows"))]
impl Notifier for StubNotifier {
    fn info(&self, title: &str, message: &str) {
        println!("[INFO] {}: {}", title, message);
    }

    fn warn(&self, title: &str, message: &str) {
        println!("[WARN] {}: {}", title, message);
    }
}

// Windows implementation only compiled if feature enabled
#[cfg(all(target_os = "windows", feature = "windows-notifications"))]
pub struct WindowsNotifier;

#[cfg(all(target_os = "windows", feature = "windows-notifications"))]
impl WindowsNotifier {
    pub fn new() -> Self {
        WindowsNotifier
    }
}

#[cfg(all(target_os = "windows", feature = "windows-notifications"))]
impl Notifier for WindowsNotifier {
    fn info(&self, title: &str, message: &str) {
        use windows::UI::Notifications::*;
        use windows::Data::Xml::Dom::*;

        let toast_xml = ToastNotificationManager::GetTemplateContent(ToastTemplateType::ToastText02)
            .expect("Failed to create toast XML");
        let nodes = toast_xml.GetElementsByTagName("text").unwrap();
        nodes.Item(0).unwrap().AppendChild(&toast_xml.CreateTextNode(title).unwrap()).unwrap();
        nodes.Item(1).unwrap().AppendChild(&toast_xml.CreateTextNode(message).unwrap()).unwrap();

        let notifier = ToastNotificationManager::CreateToastNotifierWithId("MyUpdater").unwrap();
        let toast = ToastNotification::CreateToastNotification(&toast_xml).unwrap();
        notifier.Show(&toast).unwrap();
    }

    fn warn(&self, title: &str, message: &str) {
        self.info(title, message);
    }
}
