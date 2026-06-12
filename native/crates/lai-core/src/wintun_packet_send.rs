use crate::virtual_packet_io::{build_ipv4_udp_packet, VirtualUdpPacket};
use serde::{Deserialize, Serialize};

const MIN_RING_CAPACITY: u32 = 128 * 1024;
const MAX_RING_CAPACITY: u32 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunPacketSendProbeRequest {
    pub adapter_name: String,
    pub ring_capacity: u32,
    pub packet: VirtualUdpPacket,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunPacketSendProbeReport {
    pub status: String,
    pub adapter_name: Option<String>,
    pub ring_capacity: u32,
    pub opened: bool,
    pub session_started: bool,
    pub packet_allocated: bool,
    pub packet_sent: bool,
    pub session_ended: bool,
    pub closed: bool,
    pub packet_bytes: usize,
    pub error: Option<String>,
}

pub fn probe_wintun_packet_send(
    request: WintunPacketSendProbeRequest,
) -> WintunPacketSendProbeReport {
    let packet_bytes = match build_ipv4_udp_packet(&request.packet, 64) {
        Ok(packet_bytes) => packet_bytes,
        Err(error) => return failed_before_open("invalid-packet", request, 0, error),
    };

    if !valid_ring_capacity(request.ring_capacity) {
        return failed_before_open(
            "invalid-capacity",
            request,
            packet_bytes.len(),
            "Wintun ring capacity must be a power of two between 131072 and 67108864 bytes."
                .to_owned(),
        );
    }

    #[cfg(windows)]
    {
        probe_wintun_packet_send_windows(request, packet_bytes)
    }
    #[cfg(not(windows))]
    {
        WintunPacketSendProbeReport {
            status: "unsupported".to_owned(),
            adapter_name: None,
            ring_capacity: request.ring_capacity,
            opened: false,
            session_started: false,
            packet_allocated: false,
            packet_sent: false,
            session_ended: false,
            closed: false,
            packet_bytes: packet_bytes.len(),
            error: Some("Wintun packet send is only supported on Windows".to_owned()),
        }
    }
}

fn valid_ring_capacity(ring_capacity: u32) -> bool {
    (MIN_RING_CAPACITY..=MAX_RING_CAPACITY).contains(&ring_capacity)
        && ring_capacity.is_power_of_two()
}

fn failed_before_open(
    status: &str,
    request: WintunPacketSendProbeRequest,
    packet_bytes: usize,
    error: String,
) -> WintunPacketSendProbeReport {
    WintunPacketSendProbeReport {
        status: status.to_owned(),
        adapter_name: None,
        ring_capacity: request.ring_capacity,
        opened: false,
        session_started: false,
        packet_allocated: false,
        packet_sent: false,
        session_ended: false,
        closed: false,
        packet_bytes,
        error: Some(error),
    }
}

#[cfg(windows)]
fn probe_wintun_packet_send_windows(
    request: WintunPacketSendProbeRequest,
    packet_bytes: Vec<u8>,
) -> WintunPacketSendProbeReport {
    use std::ffi::{c_void, CString};
    use std::ptr;
    use winapi::um::libloaderapi::{FreeLibrary, LoadLibraryA};

    unsafe {
        let dll_name = CString::new("wintun.dll").unwrap();
        let dll = LoadLibraryA(dll_name.as_ptr());
        if dll.is_null() {
            return WintunPacketSendProbeReport {
                status: "dll-not-found".to_owned(),
                adapter_name: None,
                ring_capacity: request.ring_capacity,
                opened: false,
                session_started: false,
                packet_allocated: false,
                packet_sent: false,
                session_ended: false,
                closed: false,
                packet_bytes: packet_bytes.len(),
                error: Some("wintun.dll not found".to_owned()),
            };
        }

        let open_func = load_wintun_function(dll, "WintunOpenAdapter");
        let close_func = load_wintun_function(dll, "WintunCloseAdapter");
        let start_func = load_wintun_function(dll, "WintunStartSession");
        let end_func = load_wintun_function(dll, "WintunEndSession");
        let allocate_func = load_wintun_function(dll, "WintunAllocateSendPacket");
        let send_func = load_wintun_function(dll, "WintunSendPacket");
        let (open_func, close_func, start_func, end_func, allocate_func, send_func) = match (
            open_func,
            close_func,
            start_func,
            end_func,
            allocate_func,
            send_func,
        ) {
            (
                Some(open_func),
                Some(close_func),
                Some(start_func),
                Some(end_func),
                Some(allocate_func),
                Some(send_func),
            ) => (
                open_func,
                close_func,
                start_func,
                end_func,
                allocate_func,
                send_func,
            ),
            _ => {
                FreeLibrary(dll);
                return WintunPacketSendProbeReport {
                    status: "function-not-found".to_owned(),
                    adapter_name: None,
                    ring_capacity: request.ring_capacity,
                    opened: false,
                    session_started: false,
                    packet_allocated: false,
                    packet_sent: false,
                    session_ended: false,
                    closed: false,
                    packet_bytes: packet_bytes.len(),
                    error: Some("Required Wintun packet send function not found".to_owned()),
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
        type WintunAllocateSendPacketFn = unsafe extern "system" fn(*mut c_void, u32) -> *mut u8;
        type WintunSendPacketFn = unsafe extern "system" fn(*mut c_void, *mut u8);

        let open_adapter: WintunOpenAdapterFn = std::mem::transmute(open_func);
        let close_adapter: WintunCloseAdapterFn = std::mem::transmute(close_func);
        let start_session: WintunStartSessionFn = std::mem::transmute(start_func);
        let end_session: WintunEndSessionFn = std::mem::transmute(end_func);
        let allocate_send_packet: WintunAllocateSendPacketFn = std::mem::transmute(allocate_func);
        let send_packet: WintunSendPacketFn = std::mem::transmute(send_func);

        let adapter = open_adapter(adapter_name_wide.as_ptr());
        if adapter.is_null() {
            FreeLibrary(dll);
            return WintunPacketSendProbeReport {
                status: "open-failed".to_owned(),
                adapter_name: None,
                ring_capacity: request.ring_capacity,
                opened: false,
                session_started: false,
                packet_allocated: false,
                packet_sent: false,
                session_ended: false,
                closed: false,
                packet_bytes: packet_bytes.len(),
                error: Some("WintunOpenAdapter returned null".to_owned()),
            };
        }

        let session = start_session(adapter, request.ring_capacity);
        if session.is_null() {
            close_adapter(adapter);
            FreeLibrary(dll);
            return WintunPacketSendProbeReport {
                status: "session-start-failed".to_owned(),
                adapter_name: Some(request.adapter_name),
                ring_capacity: request.ring_capacity,
                opened: true,
                session_started: false,
                packet_allocated: false,
                packet_sent: false,
                session_ended: false,
                closed: true,
                packet_bytes: packet_bytes.len(),
                error: Some("WintunStartSession returned null".to_owned()),
            };
        }

        let packet = allocate_send_packet(session, packet_bytes.len() as u32);
        if packet.is_null() {
            end_session(session);
            close_adapter(adapter);
            FreeLibrary(dll);
            return WintunPacketSendProbeReport {
                status: "packet-allocate-failed".to_owned(),
                adapter_name: Some(request.adapter_name),
                ring_capacity: request.ring_capacity,
                opened: true,
                session_started: true,
                packet_allocated: false,
                packet_sent: false,
                session_ended: true,
                closed: true,
                packet_bytes: packet_bytes.len(),
                error: Some("WintunAllocateSendPacket returned null".to_owned()),
            };
        }

        ptr::copy_nonoverlapping(packet_bytes.as_ptr(), packet, packet_bytes.len());
        send_packet(session, packet);
        end_session(session);
        close_adapter(adapter);
        FreeLibrary(dll);

        WintunPacketSendProbeReport {
            status: "packet-sent".to_owned(),
            adapter_name: Some(request.adapter_name),
            ring_capacity: request.ring_capacity,
            opened: true,
            session_started: true,
            packet_allocated: true,
            packet_sent: true,
            session_ended: true,
            closed: true,
            packet_bytes: packet_bytes.len(),
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
    fn rejects_invalid_ring_capacity_before_open() {
        let report = probe_wintun_packet_send(WintunPacketSendProbeRequest {
            adapter_name: "LocalAreaInterconnection".to_owned(),
            ring_capacity: 1000,
            packet: VirtualUdpPacket {
                source_ip: "10.77.12.2".parse().unwrap(),
                destination_ip: "10.77.12.255".parse().unwrap(),
                source_port: 39077,
                destination_port: 27015,
                payload: b"probe".to_vec(),
                broadcast: true,
            },
        });

        assert_eq!(report.status, "invalid-capacity");
        assert_eq!(report.packet_bytes, 33);
        assert!(!report.opened);
        assert!(!report.packet_sent);
    }
}
