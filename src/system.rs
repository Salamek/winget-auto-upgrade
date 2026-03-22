pub trait System {
    fn is_metered_connection(&self) -> bool;
    fn is_running_as_system(&self) -> bool;
    fn has_active_user_session(&self) -> bool;
}

#[cfg(target_os = "windows")]
pub struct WindowsSystem;

#[cfg(target_os = "windows")]
impl WindowsSystem {
    pub fn new() -> Self {
        WindowsSystem
    }
}

#[cfg(target_os = "windows")]
impl System for WindowsSystem {
    fn has_active_user_session(&self) -> bool {
        use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;
        unsafe { WTSGetActiveConsoleSessionId() != 0xFFFF_FFFF }
    }

    fn is_metered_connection(&self) -> bool {
        use windows::Networking::Connectivity::{NetworkCostType, NetworkInformation};

        let profile = match NetworkInformation::GetInternetConnectionProfile() {
            Ok(p) => p,
            Err(_) => return false,
        };
        let cost = match profile.GetConnectionCost() {
            Ok(c) => c,
            Err(_) => return false,
        };
        matches!(
            cost.NetworkCostType(),
            Ok(NetworkCostType::Fixed) | Ok(NetworkCostType::Variable)
        )
    }

    fn is_running_as_system(&self) -> bool {
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::Security::{
            GetTokenInformation, IsWellKnownSid, TokenUser, TOKEN_QUERY, TOKEN_USER,
            WinLocalSystemSid,
        };
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

        unsafe {
            let mut token = HANDLE::default();
            if !OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).as_bool() {
                return false;
            }

            let mut len = 0u32;
            let _ = GetTokenInformation(token, TokenUser, None, 0, &mut len);

            let mut buf = vec![0u8; len as usize];
            let ok = GetTokenInformation(
                token,
                TokenUser,
                Some(buf.as_mut_ptr() as *mut _),
                len,
                &mut len,
            );
            let _ = CloseHandle(token);

            if !ok.as_bool() {
                return false;
            }

            let token_user = &*(buf.as_ptr() as *const TOKEN_USER);
            IsWellKnownSid(token_user.User.Sid, WinLocalSystemSid).as_bool()
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub struct StubSystem;

#[cfg(not(target_os = "windows"))]
impl StubSystem {
    pub fn new() -> Self {
        StubSystem
    }
}

#[cfg(not(target_os = "windows"))]
impl System for StubSystem {
    fn has_active_user_session(&self) -> bool {
        false
    }

    fn is_metered_connection(&self) -> bool {
        false
    }

    fn is_running_as_system(&self) -> bool {
        false
    }
}
