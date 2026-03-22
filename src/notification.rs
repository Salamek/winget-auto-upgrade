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
        if matches!(self.level, NotificationLevel::All | NotificationLevel::Success) {
            println!("[SUCCESS] {}: {}", title, message);
        }
    }

    fn warning(&self, title: &str, message: &str) {
        if matches!(self.level, NotificationLevel::All | NotificationLevel::Error) {
            println!("[WARNING] {}: {}", title, message);
        }
    }

    fn error(&self, title: &str, message: &str) {
        if matches!(self.level, NotificationLevel::All | NotificationLevel::Error) {
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
    is_system: bool,
}

#[cfg(target_os = "windows")]
impl WindowsNotifier {
    pub fn new(level: NotificationLevel, is_system: bool) -> Self {
        use windows::UI::Notifications::ToastNotificationManager;
        use windows::core::HSTRING;

        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
            "Windows.SystemToast.WAUG.Notification",
        ))
        .expect("Failed to create toast notifier");

        let icon_dir = Self::resolve_icon_dir();

        WindowsNotifier { level, notifier, icon_dir, is_system }
    }

    /// Called by the `notify` subcommand — shows a toast unconditionally in the
    /// current (user) session without any level check or SYSTEM relay.
    pub fn show_for_notify(title: &str, message: &str, icon: &str) {
        use windows::UI::Notifications::ToastNotificationManager;
        use windows::core::HSTRING;

        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
            "Windows.SystemToast.WAUG.Notification",
        ))
        .expect("Failed to create toast notifier");

        let tmp = WindowsNotifier {
            level: NotificationLevel::All,
            notifier,
            icon_dir: Self::resolve_icon_dir(),
            is_system: false,
        };
        tmp.show(title, message, icon);
    }

    fn resolve_icon_dir() -> std::path::PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_default()
            .join("assets")
            .join("icons")
    }

    fn show(&self, title: &str, message: &str, icon: &str) {
        // When running as SYSTEM (session 0), toasts are invisible to the logged-in
        // user. Relay to the user session by spawning ourselves with `notify` args.
        if self.is_system {
            self.spawn_notify_as_user(title, message, icon);
            return;
        }

        use windows::Data::Xml::Dom::XmlDocument;
        use windows::UI::Notifications::ToastNotification;
        use windows::core::HSTRING;

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
        doc.LoadXml(&HSTRING::from(xml.as_str())).expect("LoadXml failed");

        let toast = ToastNotification::CreateToastNotification(&doc).unwrap();
        toast.SetTag(&HSTRING::from("winget-autoupgrade-status")).unwrap();
        self.notifier.Show(&toast).unwrap();
    }

    /// Spawn ourselves with the `notify` subcommand in the active console user's
    /// session using WTSQueryUserToken + CreateProcessAsUserW.
    fn spawn_notify_as_user(&self, title: &str, message: &str, icon: &str) {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::Security::{DuplicateTokenEx, SecurityImpersonation, TOKEN_ALL_ACCESS, TokenPrimary};
        use windows::Win32::System::RemoteDesktop::{WTSGetActiveConsoleSessionId, WTSQueryUserToken};
        use windows::Win32::System::Threading::{
            CreateProcessAsUserW, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW,
        };
        use windows::core::PWSTR;

        unsafe {
            let session_id = WTSGetActiveConsoleSessionId();
            if session_id == 0xFFFF_FFFF {
                log::warn!("No active console session; skipping user notification");
                return;
            }

            let mut user_token = windows::Win32::Foundation::HANDLE::default();
            if !WTSQueryUserToken(session_id, &mut user_token).as_bool() {
                log::warn!("WTSQueryUserToken failed");
                return;
            }

            let mut primary_token = windows::Win32::Foundation::HANDLE::default();
            if !DuplicateTokenEx(
                user_token,
                TOKEN_ALL_ACCESS,
                None,
                SecurityImpersonation,
                TokenPrimary,
                &mut primary_token,
            ).as_bool() {
                log::warn!("DuplicateTokenEx failed");
                CloseHandle(user_token);
                return;
            }
            CloseHandle(user_token);

            let exe = match std::env::current_exe() {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("current_exe failed: {e}");
                    CloseHandle(primary_token);
                    return;
                }
            };

            let cmd = format!(
                "{} notify --title {} --message {} --icon {}",
                win_quote(&exe.to_string_lossy()),
                win_quote(title),
                win_quote(message),
                win_quote(icon),
            );

            let mut cmd_wide: Vec<u16> = cmd.encode_utf16().chain(Some(0)).collect();
            let si = STARTUPINFOW {
                cb: std::mem::size_of::<STARTUPINFOW>() as u32,
                ..Default::default()
            };
            let mut pi = PROCESS_INFORMATION::default();

            if !CreateProcessAsUserW(
                primary_token,
                None,
                PWSTR(cmd_wide.as_mut_ptr()),
                None,
                None,
                false,
                PROCESS_CREATION_FLAGS(0),
                None,
                None,
                &si,
                &mut pi,
            ).as_bool() {
                log::warn!("CreateProcessAsUserW failed");
            } else {
                CloseHandle(pi.hProcess);
                CloseHandle(pi.hThread);
            }

            CloseHandle(primary_token);
        }
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
        if matches!(self.level, NotificationLevel::All | NotificationLevel::Success) {
            self.show(title, message, "success");
        }
    }

    fn warning(&self, title: &str, message: &str) {
        if matches!(self.level, NotificationLevel::All | NotificationLevel::Error) {
            self.show(title, message, "warning");
        }
    }

    fn error(&self, title: &str, message: &str) {
        if matches!(self.level, NotificationLevel::All | NotificationLevel::Error) {
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

/// Quote a string for use as a Windows CreateProcess command-line argument.
/// Implements the rules from CommandLineToArgvW: backslashes before a quote or
/// at the end of the string are doubled; embedded quotes are backslash-escaped.
fn win_quote(s: &str) -> String {
    let mut out = String::from('"');
    let mut backslashes: usize = 0;
    for c in s.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                for _ in 0..backslashes * 2 {
                    out.push('\\');
                }
                backslashes = 0;
                out.push_str("\\\"");
            }
            _ => {
                for _ in 0..backslashes {
                    out.push('\\');
                }
                backslashes = 0;
                out.push(c);
            }
        }
    }
    // Double trailing backslashes before the closing quote.
    for _ in 0..backslashes * 2 {
        out.push('\\');
    }
    out.push('"');
    out
}
