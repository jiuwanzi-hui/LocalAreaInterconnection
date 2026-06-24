use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunAdapterCreateRequest {
    pub adapter_name: String,
    pub tunnel_type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunAdapterCreateReport {
    pub status: String,
    pub adapter_name: Option<String>,
    pub reboot_required: Option<bool>,
    pub error: Option<String>,
}

pub fn create_wintun_adapter(request: WintunAdapterCreateRequest) -> WintunAdapterCreateReport {
    #[cfg(windows)]
    {
        create_wintun_adapter_windows(request)
    }
    #[cfg(not(windows))]
    {
        WintunAdapterCreateReport {
            status: "unsupported".to_owned(),
            adapter_name: None,
            reboot_required: None,
            error: Some("Wintun adapters are only supported on Windows".to_owned()),
        }
    }
}

#[cfg(windows)]
fn create_wintun_adapter_windows(request: WintunAdapterCreateRequest) -> WintunAdapterCreateReport {
    use std::ffi::CString;
    use std::ptr;
    use winapi::shared::guiddef::GUID;
    use winapi::um::libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryA};

    unsafe {
        let dll_name = CString::new("wintun.dll").unwrap();
        let dll = LoadLibraryA(dll_name.as_ptr());
        if dll.is_null() {
            return WintunAdapterCreateReport {
                status: "dll-not-found".to_owned(),
                adapter_name: None,
                reboot_required: None,
                error: Some("wintun.dll not found".to_owned()),
            };
        }

        let create_func_name = CString::new("WintunCreateAdapter").unwrap();
        let create_func = GetProcAddress(dll, create_func_name.as_ptr());
        if create_func.is_null() {
            FreeLibrary(dll);
            return WintunAdapterCreateReport {
                status: "function-not-found".to_owned(),
                adapter_name: None,
                reboot_required: None,
                error: Some("WintunCreateAdapter function not found".to_owned()),
            };
        }
        let close_func_name = CString::new("WintunCloseAdapter").unwrap();
        let close_func = GetProcAddress(dll, close_func_name.as_ptr());
        if close_func.is_null() {
            FreeLibrary(dll);
            return WintunAdapterCreateReport {
                status: "function-not-found".to_owned(),
                adapter_name: None,
                reboot_required: None,
                error: Some("WintunCloseAdapter function not found".to_owned()),
            };
        }

        let adapter_name_wide: Vec<u16> = request
            .adapter_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let tunnel_type_wide: Vec<u16> = request
            .tunnel_type
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let guid = ptr::null::<GUID>();
        type WintunCreateAdapterFn =
            unsafe extern "system" fn(*const u16, *const u16, *const GUID) -> *mut std::ffi::c_void;
        let create_adapter: WintunCreateAdapterFn = std::mem::transmute(create_func);
        let adapter = create_adapter(adapter_name_wide.as_ptr(), tunnel_type_wide.as_ptr(), guid);

        if adapter.is_null() {
            let error = std::io::Error::last_os_error();
            FreeLibrary(dll);
            WintunAdapterCreateReport {
                status: "create-failed".to_owned(),
                adapter_name: None,
                reboot_required: None,
                error: Some(format!("WintunCreateAdapter returned null ({error}).")),
            }
        } else {
            type WintunCloseAdapterFn = unsafe extern "system" fn(*mut std::ffi::c_void);
            let close_adapter: WintunCloseAdapterFn = std::mem::transmute(close_func);
            close_adapter(adapter);
            FreeLibrary(dll);
            WintunAdapterCreateReport {
                status: "created".to_owned(),
                adapter_name: Some(request.adapter_name),
                reboot_required: None,
                error: None,
            }
        }
    }
}
