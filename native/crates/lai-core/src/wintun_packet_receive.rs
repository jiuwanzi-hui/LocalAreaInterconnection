use crate::virtual_packet_io::parse_ipv4_udp_packet;
use serde::{Deserialize, Serialize};

const MIN_RING_CAPACITY: u32 = 128 * 1024;
const MAX_RING_CAPACITY: u32 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunPacketReceiveProbeRequest {
    pub adapter_name: String,
    pub ring_capacity: u32,
    pub max_attempts: u32,
    pub poll_interval_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunReceivedPacketSummary {
    pub packet_bytes: usize,
    pub source_ip: Option<String>,
    pub destination_ip: Option<String>,
    pub source_port: Option<u16>,
    pub destination_port: Option<u16>,
    pub payload_bytes: Option<usize>,
    pub broadcast: Option<bool>,
    pub parse_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WintunPacketReceiveProbeReport {
    pub status: String,
    pub adapter_name: Option<String>,
    pub ring_capacity: u32,
    pub max_attempts: u32,
    pub poll_interval_ms: u64,
    pub opened: bool,
    pub session_started: bool,
    pub receive_attempts: u32,
    pub packet_received: bool,
    pub packet_released: bool,
    pub session_ended: bool,
    pub closed: bool,
    pub packet: Option<WintunReceivedPacketSummary>,
    pub error: Option<String>,
}

pub fn probe_wintun_packet_receive(
    request: WintunPacketReceiveProbeRequest,
) -> WintunPacketReceiveProbeReport {
    if !valid_ring_capacity(request.ring_capacity) {
        return failed_before_open(
            "invalid-capacity",
            request,
            "Wintun ring capacity must be a power of two between 131072 and 67108864 bytes."
                .to_owned(),
        );
    }
    if request.max_attempts == 0 {
        return failed_before_open(
            "invalid-attempts",
            request,
            "Receive probe max_attempts must be greater than zero.".to_owned(),
        );
    }

    #[cfg(windows)]
    {
        probe_wintun_packet_receive_windows(request)
    }
    #[cfg(not(windows))]
    {
        WintunPacketReceiveProbeReport {
            status: "unsupported".to_owned(),
            adapter_name: None,
            ring_capacity: request.ring_capacity,
            max_attempts: request.max_attempts,
            poll_interval_ms: request.poll_interval_ms,
            opened: false,
            session_started: false,
            receive_attempts: 0,
            packet_received: false,
            packet_released: false,
            session_ended: false,
            closed: false,
            packet: None,
            error: Some("Wintun packet receive is only supported on Windows".to_owned()),
        }
    }
}

fn valid_ring_capacity(ring_capacity: u32) -> bool {
    (MIN_RING_CAPACITY..=MAX_RING_CAPACITY).contains(&ring_capacity)
        && ring_capacity.is_power_of_two()
}

fn failed_before_open(
    status: &str,
    request: WintunPacketReceiveProbeRequest,
    error: String,
) -> WintunPacketReceiveProbeReport {
    WintunPacketReceiveProbeReport {
        status: status.to_owned(),
        adapter_name: None,
        ring_capacity: request.ring_capacity,
        max_attempts: request.max_attempts,
        poll_interval_ms: request.poll_interval_ms,
        opened: false,
        session_started: false,
        receive_attempts: 0,
        packet_received: false,
        packet_released: false,
        session_ended: false,
        closed: false,
        packet: None,
        error: Some(error),
    }
}

fn summarize_packet(bytes: &[u8]) -> WintunReceivedPacketSummary {
    match parse_ipv4_udp_packet(bytes) {
        Ok(packet) => WintunReceivedPacketSummary {
            packet_bytes: bytes.len(),
            source_ip: Some(packet.source_ip.to_string()),
            destination_ip: Some(packet.destination_ip.to_string()),
            source_port: Some(packet.source_port),
            destination_port: Some(packet.destination_port),
            payload_bytes: Some(packet.payload.len()),
            broadcast: Some(packet.broadcast),
            parse_error: None,
        },
        Err(error) => WintunReceivedPacketSummary {
            packet_bytes: bytes.len(),
            source_ip: None,
            destination_ip: None,
            source_port: None,
            destination_port: None,
            payload_bytes: None,
            broadcast: None,
            parse_error: Some(error),
        },
    }
}

#[cfg(windows)]
fn probe_wintun_packet_receive_windows(
    request: WintunPacketReceiveProbeRequest,
) -> WintunPacketReceiveProbeReport {
    use std::ffi::{c_void, CString};
    use std::slice;
    use std::thread;
    use std::time::Duration;
    use winapi::um::libloaderapi::{FreeLibrary, LoadLibraryA};

    unsafe {
        let dll_name = CString::new("wintun.dll").unwrap();
        let dll = LoadLibraryA(dll_name.as_ptr());
        if dll.is_null() {
            return WintunPacketReceiveProbeReport {
                status: "dll-not-found".to_owned(),
                adapter_name: None,
                ring_capacity: request.ring_capacity,
                max_attempts: request.max_attempts,
                poll_interval_ms: request.poll_interval_ms,
                opened: false,
                session_started: false,
                receive_attempts: 0,
                packet_received: false,
                packet_released: false,
                session_ended: false,
                closed: false,
                packet: None,
                error: Some("wintun.dll not found".to_owned()),
            };
        }

        let open_func = load_wintun_function(dll, "WintunOpenAdapter");
        let close_func = load_wintun_function(dll, "WintunCloseAdapter");
        let start_func = load_wintun_function(dll, "WintunStartSession");
        let end_func = load_wintun_function(dll, "WintunEndSession");
        let receive_func = load_wintun_function(dll, "WintunReceivePacket");
        let release_func = load_wintun_function(dll, "WintunReleaseReceivePacket");
        let (open_func, close_func, start_func, end_func, receive_func, release_func) = match (
            open_func,
            close_func,
            start_func,
            end_func,
            receive_func,
            release_func,
        ) {
            (
                Some(open_func),
                Some(close_func),
                Some(start_func),
                Some(end_func),
                Some(receive_func),
                Some(release_func),
            ) => (
                open_func,
                close_func,
                start_func,
                end_func,
                receive_func,
                release_func,
            ),
            _ => {
                FreeLibrary(dll);
                return WintunPacketReceiveProbeReport {
                    status: "function-not-found".to_owned(),
                    adapter_name: None,
                    ring_capacity: request.ring_capacity,
                    max_attempts: request.max_attempts,
                    poll_interval_ms: request.poll_interval_ms,
                    opened: false,
                    session_started: false,
                    receive_attempts: 0,
                    packet_received: false,
                    packet_released: false,
                    session_ended: false,
                    closed: false,
                    packet: None,
                    error: Some("Required Wintun packet receive function not found".to_owned()),
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
        type WintunReceivePacketFn = unsafe extern "system" fn(*mut c_void, *mut u32) -> *mut u8;
        type WintunReleaseReceivePacketFn = unsafe extern "system" fn(*mut c_void, *mut u8);

        let open_adapter: WintunOpenAdapterFn = std::mem::transmute(open_func);
        let close_adapter: WintunCloseAdapterFn = std::mem::transmute(close_func);
        let start_session: WintunStartSessionFn = std::mem::transmute(start_func);
        let end_session: WintunEndSessionFn = std::mem::transmute(end_func);
        let receive_packet: WintunReceivePacketFn = std::mem::transmute(receive_func);
        let release_receive_packet: WintunReleaseReceivePacketFn =
            std::mem::transmute(release_func);

        let adapter = open_adapter(adapter_name_wide.as_ptr());
        if adapter.is_null() {
            FreeLibrary(dll);
            return WintunPacketReceiveProbeReport {
                status: "open-failed".to_owned(),
                adapter_name: None,
                ring_capacity: request.ring_capacity,
                max_attempts: request.max_attempts,
                poll_interval_ms: request.poll_interval_ms,
                opened: false,
                session_started: false,
                receive_attempts: 0,
                packet_received: false,
                packet_released: false,
                session_ended: false,
                closed: false,
                packet: None,
                error: Some("WintunOpenAdapter returned null".to_owned()),
            };
        }

        let session = start_session(adapter, request.ring_capacity);
        if session.is_null() {
            close_adapter(adapter);
            FreeLibrary(dll);
            return WintunPacketReceiveProbeReport {
                status: "session-start-failed".to_owned(),
                adapter_name: Some(request.adapter_name),
                ring_capacity: request.ring_capacity,
                max_attempts: request.max_attempts,
                poll_interval_ms: request.poll_interval_ms,
                opened: true,
                session_started: false,
                receive_attempts: 0,
                packet_received: false,
                packet_released: false,
                session_ended: false,
                closed: true,
                packet: None,
                error: Some("WintunStartSession returned null".to_owned()),
            };
        }

        let mut attempts = 0u32;
        let mut packet_summary = None;
        while attempts < request.max_attempts {
            attempts += 1;
            let mut packet_size = 0u32;
            let packet = receive_packet(session, &mut packet_size);
            if !packet.is_null() {
                let bytes = slice::from_raw_parts(packet, packet_size as usize).to_vec();
                release_receive_packet(session, packet);
                packet_summary = Some(summarize_packet(&bytes));
                break;
            }
            if request.poll_interval_ms > 0 && attempts < request.max_attempts {
                thread::sleep(Duration::from_millis(request.poll_interval_ms));
            }
        }

        end_session(session);
        close_adapter(adapter);
        FreeLibrary(dll);

        let packet_received = packet_summary.is_some();
        WintunPacketReceiveProbeReport {
            status: if packet_received {
                "packet-received"
            } else {
                "empty"
            }
            .to_owned(),
            adapter_name: Some(request.adapter_name),
            ring_capacity: request.ring_capacity,
            max_attempts: request.max_attempts,
            poll_interval_ms: request.poll_interval_ms,
            opened: true,
            session_started: true,
            receive_attempts: attempts,
            packet_received,
            packet_released: packet_received,
            session_ended: true,
            closed: true,
            packet: packet_summary,
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
    fn rejects_zero_attempts_before_open() {
        let report = probe_wintun_packet_receive(WintunPacketReceiveProbeRequest {
            adapter_name: "LocalAreaInterconnection".to_owned(),
            ring_capacity: 128 * 1024,
            max_attempts: 0,
            poll_interval_ms: 1,
        });

        assert_eq!(report.status, "invalid-attempts");
        assert!(!report.opened);
    }

    #[test]
    fn summarizes_ipv4_udp_packet() {
        let bytes = crate::build_ipv4_udp_packet(
            &crate::VirtualUdpPacket {
                source_ip: "10.77.12.2".parse().unwrap(),
                destination_ip: "10.77.12.255".parse().unwrap(),
                source_port: 39077,
                destination_port: 27015,
                payload: b"probe".to_vec(),
                broadcast: true,
            },
            64,
        )
        .unwrap();

        let summary = summarize_packet(&bytes);

        assert_eq!(summary.packet_bytes, 33);
        assert_eq!(summary.source_ip.as_deref(), Some("10.77.12.2"));
        assert_eq!(summary.destination_ip.as_deref(), Some("10.77.12.255"));
        assert_eq!(summary.payload_bytes, Some(5));
        assert_eq!(summary.broadcast, Some(true));
        assert!(summary.parse_error.is_none());
    }
}
