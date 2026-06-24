use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunAdapterOpenRequest {
    pub adapter_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunAdapterOpenReport {
    pub status: String,
    pub adapter_name: Option<String>,
    pub opened: bool,
    pub closed: bool,
    pub error: Option<String>,
}

pub fn open_wintun_adapter(request: WintunAdapterOpenRequest) -> WintunAdapterOpenReport {
    #[cfg(windows)]
    {
        open_wintun_adapter_windows(request)
    }
    #[cfg(not(windows))]
    {
        WintunAdapterOpenReport {
            status: "unsupported".to_owned(),
            adapter_name: None,
            opened: false,
            closed: false,
            error: Some("Wintun adapters are only supported on Windows".to_owned()),
        }
    }
}

#[cfg(windows)]
fn open_wintun_adapter_windows(request: WintunAdapterOpenRequest) -> WintunAdapterOpenReport {
    use std::ffi::CString;
    use winapi::um::libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryA};

    unsafe {
        let dll_name = CString::new("wintun.dll").unwrap();
        let dll = LoadLibraryA(dll_name.as_ptr());
        if dll.is_null() {
            return WintunAdapterOpenReport {
                status: "dll-not-found".to_owned(),
                adapter_name: None,
                opened: false,
                closed: false,
                error: Some("wintun.dll not found".to_owned()),
            };
        }

        let open_func_name = CString::new("WintunOpenAdapter").unwrap();
        let open_func = GetProcAddress(dll, open_func_name.as_ptr());
        if open_func.is_null() {
            FreeLibrary(dll);
            return WintunAdapterOpenReport {
                status: "function-not-found".to_owned(),
                adapter_name: None,
                opened: false,
                closed: false,
                error: Some("WintunOpenAdapter function not found".to_owned()),
            };
        }
        let close_func_name = CString::new("WintunCloseAdapter").unwrap();
        let close_func = GetProcAddress(dll, close_func_name.as_ptr());
        if close_func.is_null() {
            FreeLibrary(dll);
            return WintunAdapterOpenReport {
                status: "function-not-found".to_owned(),
                adapter_name: None,
                opened: false,
                closed: false,
                error: Some("WintunCloseAdapter function not found".to_owned()),
            };
        }

        let adapter_name_wide: Vec<u16> = request
            .adapter_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        type WintunOpenAdapterFn = unsafe extern "system" fn(*const u16) -> *mut std::ffi::c_void;
        let open_adapter: WintunOpenAdapterFn = std::mem::transmute(open_func);
        let adapter = open_adapter(adapter_name_wide.as_ptr());

        if adapter.is_null() {
            let error = std::io::Error::last_os_error();
            FreeLibrary(dll);
            WintunAdapterOpenReport {
                status: "open-failed".to_owned(),
                adapter_name: None,
                opened: false,
                closed: false,
                error: Some(format!("WintunOpenAdapter returned null ({error}).")),
            }
        } else {
            type WintunCloseAdapterFn = unsafe extern "system" fn(*mut std::ffi::c_void);
            let close_adapter: WintunCloseAdapterFn = std::mem::transmute(close_func);
            close_adapter(adapter);
            FreeLibrary(dll);
            WintunAdapterOpenReport {
                status: "opened-and-closed".to_owned(),
                adapter_name: Some(request.adapter_name),
                opened: true,
                closed: true,
                error: None,
            }
        }
    }
}
