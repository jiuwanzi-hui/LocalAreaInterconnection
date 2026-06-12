use crate::virtual_packet_io::{
    build_ipv4_udp_packet, parse_ipv4_packet_summary, parse_ipv4_tcp_packet, parse_ipv4_udp_packet,
    VirtualIpv4PacketSummary, VirtualTcpPacket, VirtualUdpPacket,
};
use serde::{Deserialize, Serialize};

const MIN_RING_CAPACITY: u32 = 128 * 1024;
const MAX_RING_CAPACITY: u32 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunPacketIoConfig {
    pub adapter_name: String,
    pub ring_capacity: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunPacketIoOpenReport {
    pub status: String,
    pub adapter_name: Option<String>,
    pub ring_capacity: u32,
    pub opened: bool,
    pub session_started: bool,
    pub closed: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunRuntimePacket {
    pub packet_bytes: usize,
    pub bytes: Vec<u8>,
    pub summary: Option<VirtualIpv4PacketSummary>,
    pub parsed_udp: Option<VirtualUdpPacket>,
    pub parsed_tcp: Option<VirtualTcpPacket>,
    pub parse_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunPacketIoCloseReport {
    pub session_ended: bool,
    pub closed: bool,
}

pub fn validate_wintun_ring_capacity(ring_capacity: u32) -> Result<(), String> {
    if (MIN_RING_CAPACITY..=MAX_RING_CAPACITY).contains(&ring_capacity)
        && ring_capacity.is_power_of_two()
    {
        Ok(())
    } else {
        Err(
            "Wintun ring capacity must be a power of two between 131072 and 67108864 bytes."
                .to_owned(),
        )
    }
}

#[cfg(windows)]
pub struct WintunPacketIoSession {
    dll: winapi::shared::minwindef::HMODULE,
    adapter: *mut std::ffi::c_void,
    session: *mut std::ffi::c_void,
    close_adapter: WintunCloseAdapterFn,
    end_session: WintunEndSessionFn,
    receive_packet: WintunReceivePacketFn,
    release_receive_packet: WintunReleaseReceivePacketFn,
    allocate_send_packet: WintunAllocateSendPacketFn,
    send_packet: WintunSendPacketFn,
    closed: bool,
}

#[cfg(windows)]
impl std::fmt::Debug for WintunPacketIoSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WintunPacketIoSession")
            .field("closed", &self.closed)
            .finish_non_exhaustive()
    }
}

#[cfg(windows)]
type WintunOpenAdapterFn = unsafe extern "system" fn(*const u16) -> *mut std::ffi::c_void;
#[cfg(windows)]
type WintunCloseAdapterFn = unsafe extern "system" fn(*mut std::ffi::c_void);
#[cfg(windows)]
type WintunStartSessionFn =
    unsafe extern "system" fn(*mut std::ffi::c_void, u32) -> *mut std::ffi::c_void;
#[cfg(windows)]
type WintunEndSessionFn = unsafe extern "system" fn(*mut std::ffi::c_void);
#[cfg(windows)]
type WintunReceivePacketFn = unsafe extern "system" fn(*mut std::ffi::c_void, *mut u32) -> *mut u8;
#[cfg(windows)]
type WintunReleaseReceivePacketFn = unsafe extern "system" fn(*mut std::ffi::c_void, *mut u8);
#[cfg(windows)]
type WintunAllocateSendPacketFn = unsafe extern "system" fn(*mut std::ffi::c_void, u32) -> *mut u8;
#[cfg(windows)]
type WintunSendPacketFn = unsafe extern "system" fn(*mut std::ffi::c_void, *mut u8);

#[cfg(windows)]
pub fn open_wintun_packet_io_session(
    config: WintunPacketIoConfig,
) -> Result<WintunPacketIoSession, WintunPacketIoOpenReport> {
    use std::ffi::CString;
    use winapi::um::libloaderapi::{FreeLibrary, LoadLibraryA};

    if let Err(error) = validate_wintun_ring_capacity(config.ring_capacity) {
        return Err(open_report(
            "invalid-capacity",
            &config,
            false,
            false,
            false,
            error,
        ));
    }

    unsafe {
        let dll_name = CString::new("wintun.dll").unwrap();
        let dll = LoadLibraryA(dll_name.as_ptr());
        if dll.is_null() {
            return Err(open_report(
                "dll-not-found",
                &config,
                false,
                false,
                false,
                "wintun.dll not found".to_owned(),
            ));
        }

        let open_func = load_wintun_function(dll, "WintunOpenAdapter");
        let close_func = load_wintun_function(dll, "WintunCloseAdapter");
        let start_func = load_wintun_function(dll, "WintunStartSession");
        let end_func = load_wintun_function(dll, "WintunEndSession");
        let receive_func = load_wintun_function(dll, "WintunReceivePacket");
        let release_func = load_wintun_function(dll, "WintunReleaseReceivePacket");
        let allocate_func = load_wintun_function(dll, "WintunAllocateSendPacket");
        let send_func = load_wintun_function(dll, "WintunSendPacket");
        let (
            open_func,
            close_func,
            start_func,
            end_func,
            receive_func,
            release_func,
            allocate_func,
            send_func,
        ) = match (
            open_func,
            close_func,
            start_func,
            end_func,
            receive_func,
            release_func,
            allocate_func,
            send_func,
        ) {
            (
                Some(open_func),
                Some(close_func),
                Some(start_func),
                Some(end_func),
                Some(receive_func),
                Some(release_func),
                Some(allocate_func),
                Some(send_func),
            ) => (
                open_func,
                close_func,
                start_func,
                end_func,
                receive_func,
                release_func,
                allocate_func,
                send_func,
            ),
            _ => {
                FreeLibrary(dll);
                return Err(open_report(
                    "function-not-found",
                    &config,
                    false,
                    false,
                    false,
                    "Required Wintun packet I/O function not found".to_owned(),
                ));
            }
        };

        let open_adapter: WintunOpenAdapterFn = std::mem::transmute(open_func);
        let close_adapter: WintunCloseAdapterFn = std::mem::transmute(close_func);
        let start_session: WintunStartSessionFn = std::mem::transmute(start_func);
        let end_session: WintunEndSessionFn = std::mem::transmute(end_func);
        let receive_packet: WintunReceivePacketFn = std::mem::transmute(receive_func);
        let release_receive_packet: WintunReleaseReceivePacketFn =
            std::mem::transmute(release_func);
        let allocate_send_packet: WintunAllocateSendPacketFn = std::mem::transmute(allocate_func);
        let send_packet: WintunSendPacketFn = std::mem::transmute(send_func);

        let adapter_name_wide: Vec<u16> = config
            .adapter_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let adapter = open_adapter(adapter_name_wide.as_ptr());
        if adapter.is_null() {
            FreeLibrary(dll);
            return Err(open_report(
                "open-failed",
                &config,
                false,
                false,
                false,
                "WintunOpenAdapter returned null".to_owned(),
            ));
        }

        let session = start_session(adapter, config.ring_capacity);
        if session.is_null() {
            close_adapter(adapter);
            FreeLibrary(dll);
            return Err(open_report(
                "session-start-failed",
                &config,
                true,
                false,
                true,
                "WintunStartSession returned null".to_owned(),
            ));
        }

        Ok(WintunPacketIoSession {
            dll,
            adapter,
            session,
            close_adapter,
            end_session,
            receive_packet,
            release_receive_packet,
            allocate_send_packet,
            send_packet,
            closed: false,
        })
    }
}

#[cfg(not(windows))]
pub struct WintunPacketIoSession;

#[cfg(not(windows))]
impl std::fmt::Debug for WintunPacketIoSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WintunPacketIoSession").finish()
    }
}

#[cfg(not(windows))]
pub fn open_wintun_packet_io_session(
    config: WintunPacketIoConfig,
) -> Result<WintunPacketIoSession, WintunPacketIoOpenReport> {
    if let Err(error) = validate_wintun_ring_capacity(config.ring_capacity) {
        return Err(open_report(
            "invalid-capacity",
            &config,
            false,
            false,
            false,
            error,
        ));
    }
    Err(open_report(
        "unsupported",
        &config,
        false,
        false,
        false,
        "Wintun packet I/O runtime is only supported on Windows".to_owned(),
    ))
}

#[cfg(windows)]
impl WintunPacketIoSession {
    pub fn receive_once(&mut self) -> Result<Option<WintunRuntimePacket>, String> {
        if self.closed {
            return Err("Wintun packet I/O session is already closed.".to_owned());
        }
        unsafe {
            let mut packet_size = 0u32;
            let packet = (self.receive_packet)(self.session, &mut packet_size);
            if packet.is_null() {
                return Ok(None);
            }
            let bytes = std::slice::from_raw_parts(packet, packet_size as usize).to_vec();
            (self.release_receive_packet)(self.session, packet);
            Ok(Some(runtime_packet_from_bytes(bytes)))
        }
    }

    pub fn send_ipv4_packet(&mut self, bytes: &[u8]) -> Result<usize, String> {
        if self.closed {
            return Err("Wintun packet I/O session is already closed.".to_owned());
        }
        if bytes.is_empty() || bytes.len() > u32::MAX as usize {
            return Err("Wintun send packet size is invalid.".to_owned());
        }
        unsafe {
            let packet = (self.allocate_send_packet)(self.session, bytes.len() as u32);
            if packet.is_null() {
                return Err("WintunAllocateSendPacket returned null.".to_owned());
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), packet, bytes.len());
            (self.send_packet)(self.session, packet);
            Ok(bytes.len())
        }
    }

    pub fn send_udp_packet(&mut self, packet: &VirtualUdpPacket) -> Result<usize, String> {
        let bytes = build_ipv4_udp_packet(packet, 64)?;
        self.send_ipv4_packet(&bytes)
    }

    pub fn close(&mut self) -> WintunPacketIoCloseReport {
        if self.closed {
            return WintunPacketIoCloseReport {
                session_ended: false,
                closed: false,
            };
        }
        unsafe {
            (self.end_session)(self.session);
            (self.close_adapter)(self.adapter);
            winapi::um::libloaderapi::FreeLibrary(self.dll);
        }
        self.closed = true;
        WintunPacketIoCloseReport {
            session_ended: true,
            closed: true,
        }
    }
}

#[cfg(not(windows))]
impl WintunPacketIoSession {
    pub fn receive_once(&mut self) -> Result<Option<WintunRuntimePacket>, String> {
        Err("Wintun packet I/O runtime is only supported on Windows".to_owned())
    }

    pub fn send_ipv4_packet(&mut self, _bytes: &[u8]) -> Result<usize, String> {
        Err("Wintun packet I/O runtime is only supported on Windows".to_owned())
    }

    pub fn send_udp_packet(&mut self, _packet: &VirtualUdpPacket) -> Result<usize, String> {
        Err("Wintun packet I/O runtime is only supported on Windows".to_owned())
    }

    pub fn close(&mut self) -> WintunPacketIoCloseReport {
        WintunPacketIoCloseReport {
            session_ended: false,
            closed: false,
        }
    }
}

#[cfg(windows)]
impl Drop for WintunPacketIoSession {
    fn drop(&mut self) {
        if !self.closed {
            let _ = self.close();
        }
    }
}

fn runtime_packet_from_bytes(bytes: Vec<u8>) -> WintunRuntimePacket {
    match parse_ipv4_packet_summary(&bytes) {
        Ok(summary) => {
            let parsed_udp = if summary.protocol_number == 17 {
                parse_ipv4_udp_packet(&bytes).ok()
            } else {
                None
            };
            let parsed_tcp = if summary.protocol_number == 6 {
                parse_ipv4_tcp_packet(&bytes).ok()
            } else {
                None
            };
            WintunRuntimePacket {
                packet_bytes: bytes.len(),
                bytes,
                summary: Some(summary),
                parsed_udp,
                parsed_tcp,
                parse_error: None,
            }
        }
        Err(error) => WintunRuntimePacket {
            packet_bytes: bytes.len(),
            bytes,
            summary: None,
            parsed_udp: None,
            parsed_tcp: None,
            parse_error: Some(error),
        },
    }
}

fn open_report(
    status: &str,
    config: &WintunPacketIoConfig,
    opened: bool,
    session_started: bool,
    closed: bool,
    error: String,
) -> WintunPacketIoOpenReport {
    WintunPacketIoOpenReport {
        status: status.to_owned(),
        adapter_name: opened.then(|| config.adapter_name.clone()),
        ring_capacity: config.ring_capacity,
        opened,
        session_started,
        closed,
        error: Some(error),
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
    fn validates_ring_capacity() {
        assert!(validate_wintun_ring_capacity(128 * 1024).is_ok());
        assert!(validate_wintun_ring_capacity(1000).is_err());
    }

    #[test]
    fn invalid_capacity_fails_before_open() {
        let report = open_wintun_packet_io_session(WintunPacketIoConfig {
            adapter_name: "LocalAreaInterconnection".to_owned(),
            ring_capacity: 1000,
        })
        .unwrap_err();

        assert_eq!(report.status, "invalid-capacity");
        assert!(!report.opened);
        assert!(!report.session_started);
    }
}
