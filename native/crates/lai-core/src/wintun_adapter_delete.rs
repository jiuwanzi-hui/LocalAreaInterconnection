use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunAdapterDeleteRequest {
    pub adapter_name: String,
    pub tunnel_type: String,
    pub force_close_sessions: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunAdapterDeleteReport {
    pub status: String,
    pub adapter_name: Option<String>,
    pub opened: bool,
    pub deleted: bool,
    pub closed: bool,
    pub reboot_required: Option<bool>,
    pub error: Option<String>,
}

pub fn delete_wintun_adapter(request: WintunAdapterDeleteRequest) -> WintunAdapterDeleteReport {
    #[cfg(windows)]
    {
        delete_wintun_adapter_windows(request)
    }
    #[cfg(not(windows))]
    {
        WintunAdapterDeleteReport {
            status: "unsupported".to_owned(),
            adapter_name: None,
            opened: false,
            deleted: false,
            closed: false,
            reboot_required: None,
            error: Some("Wintun adapters are only supported on Windows".to_owned()),
        }
    }
}

#[cfg(windows)]
fn delete_wintun_adapter_windows(request: WintunAdapterDeleteRequest) -> WintunAdapterDeleteReport {
    let _ = request.force_close_sessions;
    let _ = request.tunnel_type;
    WintunAdapterDeleteReport {
        status: "adapter-delete-api-unavailable".to_owned(),
        adapter_name: Some(request.adapter_name),
        opened: false,
        deleted: false,
        closed: false,
        reboot_required: None,
        error: Some(
            "The current public Wintun API does not export WintunDeleteAdapter; do not fake adapter deletion."
                .to_owned(),
        ),
    }
}
