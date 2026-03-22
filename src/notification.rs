use crate::config::NotificationLevel;

pub trait Notifier {
    fn info(&self, title: &str, message: &str);
    fn success(&self, title: &str, message: &str);
    fn warning(&self, title: &str, message: &str);
    fn error(&self, title: &str, message: &str);
}

// ── Stub (non-Windows) ────────────────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
pub struct StubNotifier {
    level: NotificationLevel,
}

#[cfg(not(target_os = "windows"))]
impl StubNotifier {
    pub fn new(level: NotificationLevel) -> Self {
        StubNotifier { level }
    }
}

#[cfg(not(target_os = "windows"))]
impl Notifier for StubNotifier {
    fn info(&self, title: &str, message: &str) {
        if matches!(self.level, NotificationLevel::All) {
            println!("[INFO] {}: {}", title, message);
        }
    }

    fn success(&self, title: &str, message: &str) {
        if matches!(
            self.level,
            NotificationLevel::All | NotificationLevel::Success
        ) {
            println!("[SUCCESS] {}: {}", title, message);
        }
    }

    fn warning(&self, title: &str, message: &str) {
        if matches!(
            self.level,
            NotificationLevel::All | NotificationLevel::Error
        ) {
            println!("[WARNING] {}: {}", title, message);
        }
    }

    fn error(&self, title: &str, message: &str) {
        if matches!(
            self.level,
            NotificationLevel::All | NotificationLevel::Error
        ) {
            println!("[ERROR] {}: {}", title, message);
        }
    }
}

// ── Windows ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub struct WindowsNotifier {
    level: NotificationLevel,
    notifier: windows::UI::Notifications::ToastNotifier,
    icon_dir: std::path::PathBuf,
}

#[cfg(target_os = "windows")]
impl WindowsNotifier {
    pub fn new(level: NotificationLevel) -> Self {
        use windows::UI::Notifications::ToastNotificationManager;
        use windows::core::HSTRING;

        // "Windows.SystemToast.WAU.Notification" is a known registered system ID
        // that works for unpackaged apps without any extra registration.
        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
            "Windows.SystemToast.WAU.Notification",
        ))
        .expect("Failed to create toast notifier");

        let icon_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_default()
            .join("assets")
            .join("icons");

        WindowsNotifier {
            level,
            notifier,
            icon_dir,
        }
    }

    fn show(&self, title: &str, message: &str, icon: &str) {
        use windows::Data::Xml::Dom::XmlDocument;
        use windows::UI::Notifications::ToastNotification;
        use windows::core::HSTRING;

        // Build XML from scratch, mirroring the WAU PowerShell approach.
        let icon_path = self.icon_dir.join(format!("{}.png", icon));
        let image_xml = if icon_path.exists() {
            format!(
                r#"<image placement="appLogoOverride" src="{}"/>"#,
                icon_path.to_string_lossy().replace('"', "")
            )
        } else {
            String::new()
        };

        let xml = format!(
            r#"<?xml version="1.0" encoding="utf-8"?><toast><visual><binding template="ToastGeneric">{image}<text>{title}</text><text>{message}</text></binding></visual></toast>"#,
            image = image_xml,
            title = escape_xml(title),
            message = escape_xml(message),
        );

        let doc = XmlDocument::new().expect("XmlDocument::new failed");
        doc.LoadXml(&HSTRING::from(xml.as_str()))
            .expect("LoadXml failed");

        let toast = ToastNotification::CreateToastNotification(&doc).unwrap();
        toast
            .SetTag(&HSTRING::from("winget-autoupgrade-status"))
            .unwrap();
        self.notifier.Show(&toast).unwrap();
    }
}

#[cfg(target_os = "windows")]
impl Notifier for WindowsNotifier {
    fn info(&self, title: &str, message: &str) {
        if matches!(self.level, NotificationLevel::All) {
            self.show(title, message, "info");
        }
    }

    fn success(&self, title: &str, message: &str) {
        if matches!(
            self.level,
            NotificationLevel::All | NotificationLevel::Success
        ) {
            self.show(title, message, "success");
        }
    }

    fn warning(&self, title: &str, message: &str) {
        if matches!(
            self.level,
            NotificationLevel::All | NotificationLevel::Error
        ) {
            self.show(title, message, "warning");
        }
    }

    fn error(&self, title: &str, message: &str) {
        if matches!(
            self.level,
            NotificationLevel::All | NotificationLevel::Error
        ) {
            self.show(title, message, "error");
        }
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
