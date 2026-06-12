use serde::{Deserialize, Serialize};

const MIN_RING_CAPACITY: u32 = 128 * 1024;
const MAX_RING_CAPACITY: u32 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunSessionProbeRequest {
    pub adapter_name: String,
    pub tunnel_type: String,
    pub ring_capacity: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunSessionProbeReport {
    pub status: String,
    pub adapter_name: Option<String>,
    pub ring_capacity: u32,
    pub opened: bool,
    pub session_started: bool,
    pub session_ended: bool,
    pub closed: bool,
    pub error: Option<String>,
}

pub fn probe_wintun_session(request: WintunSessionProbeRequest) -> WintunSessionProbeReport {
    if !valid_ring_capacity(request.ring_capacity) {
        return WintunSessionProbeReport {
            status: "invalid-capacity".to_owned(),
            adapter_name: None,
            ring_capacity: request.ring_capacity,
            opened: false,
            session_started: false,
            session_ended: false,
            closed: false,
            error: Some(
                "Wintun ring capacity must be a power of two between 131072 and 67108864 bytes."
                    .to_owned(),
            ),
        };
    }

    #[cfg(windows)]
    {
        probe_wintun_session_windows(request)
    }
    #[cfg(not(windows))]
    {
        WintunSessionProbeReport {
            status: "unsupported".to_owned(),
            adapter_name: None,
            ring_capacity: request.ring_capacity,
            opened: false,
            session_started: false,
            session_ended: false,
            closed: false,
            error: Some("Wintun sessions are only supported on Windows".to_owned()),
        }
    }
}

fn valid_ring_capacity(ring_capacity: u32) -> bool {
    (MIN_RING_CAPACITY..=MAX_RING_CAPACITY).contains(&ring_capacity)
        && ring_capacity.is_power_of_two()
}

#[cfg(windows)]
fn probe_wintun_session_windows(request: WintunSessionProbeRequest) -> WintunSessionProbeReport {
    use std::ffi::{c_void, CString};
    use winapi::um::libloaderapi::{FreeLibrary, LoadLibraryA};

    unsafe {
        let dll_name = CString::new("wintun.dll").unwrap();
        let dll = LoadLibraryA(dll_name.as_ptr());
        if dll.is_null() {
            return WintunSessionProbeReport {
                status: "dll-not-found".to_owned(),
                adapter_name: None,
                ring_capacity: request.ring_capacity,
                opened: false,
                session_started: false,
                session_ended: false,
                closed: false,
                error: Some("wintun.dll not found".to_owned()),
            };
        }

        let open_func = load_wintun_function(dll, "WintunOpenAdapter");
        let close_func = load_wintun_function(dll, "WintunCloseAdapter");
        let start_func = load_wintun_function(dll, "WintunStartSession");
        let end_func = load_wintun_function(dll, "WintunEndSession");
        let (open_func, close_func, start_func, end_func) =
            match (open_func, close_func, start_func, end_func) {
                (Some(open_func), Some(close_func), Some(start_func), Some(end_func)) => {
                    (open_func, close_func, start_func, end_func)
                }
                _ => {
                    FreeLibrary(dll);
                    return WintunSessionProbeReport {
                        status: "function-not-found".to_owned(),
                        adapter_name: None,
                        ring_capacity: request.ring_capacity,
                        opened: false,
                        session_started: false,
                        session_ended: false,
                        closed: false,
                        error: Some("Required Wintun session function not found".to_owned()),
                    };
                }
            };

        let adapter_name_wide: Vec<u16> = request
            .adapter_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        type WintunOpenAdapterFn = unsafe extern "system" fn(*const u16) -> *mut c_void;
        type WintunCloseAdapterFn = unsafe extern "system" fn(*mut c_void);
        type WintunStartSessionFn = unsafe extern "system" fn(*mut c_void, u32) -> *mut c_void;
        type WintunEndSessionFn = unsafe extern "system" fn(*mut c_void);

        let open_adapter: WintunOpenAdapterFn = std::mem::transmute(open_func);
        let close_adapter: WintunCloseAdapterFn = std::mem::transmute(close_func);
        let start_session: WintunStartSessionFn = std::mem::transmute(start_func);
        let end_session: WintunEndSessionFn = std::mem::transmute(end_func);

        let adapter = open_adapter(adapter_name_wide.as_ptr());
        if adapter.is_null() {
            FreeLibrary(dll);
            return WintunSessionProbeReport {
                status: "open-failed".to_owned(),
                adapter_name: None,
                ring_capacity: request.ring_capacity,
                opened: false,
                session_started: false,
                session_ended: false,
                closed: false,
                error: Some("WintunOpenAdapter returned null".to_owned()),
            };
        }

        let session = start_session(adapter, request.ring_capacity);
        if session.is_null() {
            close_adapter(adapter);
            FreeLibrary(dll);
            return WintunSessionProbeReport {
                status: "session-start-failed".to_owned(),
                adapter_name: Some(request.adapter_name),
                ring_capacity: request.ring_capacity,
                opened: true,
                session_started: false,
                session_ended: false,
                closed: true,
                error: Some("WintunStartSession returned null".to_owned()),
            };
        }

        end_session(session);
        close_adapter(adapter);
        FreeLibrary(dll);
        WintunSessionProbeReport {
            status: "session-started-and-ended".to_owned(),
            adapter_name: Some(request.adapter_name),
            ring_capacity: request.ring_capacity,
            opened: true,
            session_started: true,
            session_ended: true,
            closed: true,
            error: None,
        }
    }
}

#[cfg(windows)]
unsafe fn load_wintun_function(
    dll: winapi::shared::minwindef::HMODULE,
    name: &str,
) -> Option<winapi::shared::minwindef::FARPROC> {
    let name = std::ffi::CString::new(name).unwrap();
    let func = winapi::um::libloaderapi::GetProcAddress(dll, name.as_ptr());
    if func.is_null() {
        None
    } else {
        Some(func)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_ring_capacity() {
        let report = probe_wintun_session(WintunSessionProbeRequest {
            adapter_name: "LocalAreaInterconnection".to_owned(),
            tunnel_type: "LocalAreaInterconnection".to_owned(),
            ring_capacity: 1000,
        });

        assert_eq!(report.status, "invalid-capacity");
        assert!(!report.opened);
        assert!(!report.session_started);
    }
}
