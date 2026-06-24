use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WintunDetectReport {
    pub dll_path: Option<String>,
    pub dll_found: bool,
    pub is_admin: bool,
    pub status: String,
    pub next_actions: Vec<String>,
}

pub fn detect_wintun_availability() -> WintunDetectReport {
    let dll_path = detect_wintun_dll();
    let dll_found = dll_path.is_some();
    let is_admin = is_elevated();

    let mut next_actions = Vec::new();
    let status = if !dll_found && !is_admin {
        next_actions.push("Install Wintun driver (requires administrator)".to_owned());
        next_actions.push("Download wintun.dll from https://www.wintun.net/".to_owned());
        "dll-missing-needs-admin".to_owned()
    } else if !dll_found {
        next_actions.push("Download wintun.dll from https://www.wintun.net/".to_owned());
        next_actions.push("Place wintun.dll in the application directory or system32".to_owned());
        "dll-missing".to_owned()
    } else if !is_admin {
        next_actions.push("Run as administrator to create Wintun adapter".to_owned());
        "dll-found-needs-admin".to_owned()
    } else {
        next_actions.push("Ready to create Wintun adapter".to_owned());
        "ready".to_owned()
    };

    WintunDetectReport {
        dll_path,
        dll_found,
        is_admin,
        status,
        next_actions,
    }
}

fn detect_wintun_dll() -> Option<String> {
    let mut candidates = vec![
        "wintun.dll".to_owned(),
        "C:\\Windows\\System32\\wintun.dll".to_owned(),
        ".\\wintun.dll".to_owned(),
    ];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("wintun.dll").to_string_lossy().to_string());
        }
    }

    for candidate in candidates {
        if Path::new(&candidate).exists() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(windows)]
fn is_elevated() -> bool {
    use std::ptr;
    use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winnt::{TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};

    unsafe {
        let mut token = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut size = 0u32;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        );

        winapi::um::handleapi::CloseHandle(token);
        result != 0 && elevation.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
fn is_elevated() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wintun_detect_produces_report() {
        let report = detect_wintun_availability();
        assert!(!report.status.is_empty());
        assert!(!report.next_actions.is_empty());
    }
}
