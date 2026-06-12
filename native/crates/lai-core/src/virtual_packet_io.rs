use crate::udp_forwarding::UdpForwardObservation;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VirtualUdpPacket {
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
    pub source_port: u16,
    pub destination_port: u16,
    pub payload: Vec<u8>,
    pub broadcast: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VirtualTcpPacket {
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
    pub source_port: u16,
    pub destination_port: u16,
    pub payload: Vec<u8>,
    pub flags: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VirtualIpv4PacketSummary {
    pub protocol: String,
    pub protocol_number: u8,
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
    pub source_port: Option<u16>,
    pub destination_port: Option<u16>,
    pub payload_bytes: usize,
    pub packet_bytes: usize,
    pub broadcast: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VirtualPacketIoPlan {
    pub adapter_name: String,
    pub backend: String,
    pub mtu: u16,
    pub can_read_ipv4: bool,
    pub can_write_ipv4: bool,
    pub can_observe_udp: bool,
    pub can_forward_broadcast: bool,
    pub warnings: Vec<String>,
}

pub fn create_virtual_packet_io_plan(
    adapter_name: impl Into<String>,
    backend: impl Into<String>,
    mtu: u16,
) -> VirtualPacketIoPlan {
    let backend = backend.into();
    let mut warnings = Vec::new();
    if backend != "userspace-udp" && backend != "wintun" && backend != "tap" {
        warnings.push(
            "Unknown virtual packet I/O backend; only a planning contract was created.".to_owned(),
        );
    }
    if mtu < 1200 {
        warnings.push("MTU is below the recommended tunnel packet budget.".to_owned());
    }
    let raw_adapter_backend = backend == "wintun" || backend == "tap";

    VirtualPacketIoPlan {
        adapter_name: adapter_name.into(),
        backend,
        mtu,
        can_read_ipv4: raw_adapter_backend,
        can_write_ipv4: raw_adapter_backend,
        can_observe_udp: true,
        can_forward_broadcast: true,
        warnings,
    }
}

pub fn parse_ipv4_udp_packet(bytes: &[u8]) -> Result<VirtualUdpPacket, String> {
    let header = parse_ipv4_header(bytes)?;
    if header.protocol_number != 17 {
        return Err("IPv4 packet is not UDP.".to_owned());
    }

    let udp = &bytes[header.header_len..header.total_len];
    if udp.len() < 8 {
        return Err("IPv4 UDP packet is too short.".to_owned());
    }
    let source_port = u16::from_be_bytes([udp[0], udp[1]]);
    let destination_port = u16::from_be_bytes([udp[2], udp[3]]);
    let udp_len = u16::from_be_bytes([udp[4], udp[5]]) as usize;
    if udp_len < 8 || udp_len > udp.len() {
        return Err("UDP length is invalid.".to_owned());
    }
    let payload = udp[8..udp_len].to_vec();

    Ok(VirtualUdpPacket {
        source_ip: header.source_ip,
        destination_ip: header.destination_ip,
        source_port,
        destination_port,
        payload,
        broadcast: is_broadcast_ip(header.destination_ip),
    })
}

pub fn parse_ipv4_tcp_packet(bytes: &[u8]) -> Result<VirtualTcpPacket, String> {
    let header = parse_ipv4_header(bytes)?;
    if header.protocol_number != 6 {
        return Err("IPv4 packet is not TCP.".to_owned());
    }

    let tcp = &bytes[header.header_len..header.total_len];
    if tcp.len() < 20 {
        return Err("IPv4 TCP packet is too short.".to_owned());
    }
    let source_port = u16::from_be_bytes([tcp[0], tcp[1]]);
    let destination_port = u16::from_be_bytes([tcp[2], tcp[3]]);
    let data_offset = ((tcp[12] >> 4) as usize) * 4;
    if data_offset < 20 || data_offset > tcp.len() {
        return Err("TCP data offset is invalid.".to_owned());
    }
    let flags = (((tcp[12] & 0x01) as u16) << 8) | tcp[13] as u16;
    let payload = tcp[data_offset..].to_vec();

    Ok(VirtualTcpPacket {
        source_ip: header.source_ip,
        destination_ip: header.destination_ip,
        source_port,
        destination_port,
        payload,
        flags,
    })
}

pub fn parse_ipv4_packet_summary(bytes: &[u8]) -> Result<VirtualIpv4PacketSummary, String> {
    let header = parse_ipv4_header(bytes)?;
    match header.protocol_number {
        17 => {
            let packet = parse_ipv4_udp_packet(bytes)?;
            Ok(VirtualIpv4PacketSummary {
                protocol: "udp".to_owned(),
                protocol_number: 17,
                source_ip: packet.source_ip,
                destination_ip: packet.destination_ip,
                source_port: Some(packet.source_port),
                destination_port: Some(packet.destination_port),
                payload_bytes: packet.payload.len(),
                packet_bytes: header.total_len,
                broadcast: packet.broadcast,
            })
        }
        6 => {
            let packet = parse_ipv4_tcp_packet(bytes)?;
            Ok(VirtualIpv4PacketSummary {
                protocol: "tcp".to_owned(),
                protocol_number: 6,
                source_ip: packet.source_ip,
                destination_ip: packet.destination_ip,
                source_port: Some(packet.source_port),
                destination_port: Some(packet.destination_port),
                payload_bytes: packet.payload.len(),
                packet_bytes: header.total_len,
                broadcast: is_broadcast_ip(packet.destination_ip),
            })
        }
        1 => Ok(VirtualIpv4PacketSummary {
            protocol: "icmp".to_owned(),
            protocol_number: 1,
            source_ip: header.source_ip,
            destination_ip: header.destination_ip,
            source_port: None,
            destination_port: None,
            payload_bytes: header.total_len.saturating_sub(header.header_len),
            packet_bytes: header.total_len,
            broadcast: is_broadcast_ip(header.destination_ip),
        }),
        other => Ok(VirtualIpv4PacketSummary {
            protocol: format!("ipv4-{other}"),
            protocol_number: other,
            source_ip: header.source_ip,
            destination_ip: header.destination_ip,
            source_port: None,
            destination_port: None,
            payload_bytes: header.total_len.saturating_sub(header.header_len),
            packet_bytes: header.total_len,
            broadcast: is_broadcast_ip(header.destination_ip),
        }),
    }
}

pub fn build_ipv4_udp_packet(packet: &VirtualUdpPacket, ttl: u8) -> Result<Vec<u8>, String> {
    let total_len = 20usize
        .checked_add(8)
        .and_then(|len| len.checked_add(packet.payload.len()))
        .ok_or_else(|| "IPv4 UDP packet length overflowed.".to_owned())?;
    if total_len > u16::MAX as usize {
        return Err("IPv4 UDP packet is too large.".to_owned());
    }
    let udp_len = 8 + packet.payload.len();
    let mut bytes = vec![0u8; total_len];
    bytes[0] = 0x45;
    bytes[1] = 0;
    bytes[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
    bytes[4..6].copy_from_slice(&0u16.to_be_bytes());
    bytes[6..8].copy_from_slice(&0u16.to_be_bytes());
    bytes[8] = ttl;
    bytes[9] = 17;
    bytes[12..16].copy_from_slice(&packet.source_ip.octets());
    bytes[16..20].copy_from_slice(&packet.destination_ip.octets());
    let checksum = ipv4_header_checksum(&bytes[..20]);
    bytes[10..12].copy_from_slice(&checksum.to_be_bytes());

    bytes[20..22].copy_from_slice(&packet.source_port.to_be_bytes());
    bytes[22..24].copy_from_slice(&packet.destination_port.to_be_bytes());
    bytes[24..26].copy_from_slice(&(udp_len as u16).to_be_bytes());
    bytes[26..28].copy_from_slice(&0u16.to_be_bytes());
    bytes[28..].copy_from_slice(&packet.payload);
    Ok(bytes)
}

pub fn build_ipv4_tcp_packet(packet: &VirtualTcpPacket, ttl: u8) -> Result<Vec<u8>, String> {
    let total_len = 20usize
        .checked_add(20)
        .and_then(|len| len.checked_add(packet.payload.len()))
        .ok_or_else(|| "IPv4 TCP packet length overflowed.".to_owned())?;
    if total_len > u16::MAX as usize {
        return Err("IPv4 TCP packet is too large.".to_owned());
    }
    let mut bytes = vec![0u8; total_len];
    write_ipv4_header(
        &mut bytes,
        total_len,
        ttl,
        6,
        packet.source_ip,
        packet.destination_ip,
    );
    bytes[20..22].copy_from_slice(&packet.source_port.to_be_bytes());
    bytes[22..24].copy_from_slice(&packet.destination_port.to_be_bytes());
    bytes[24..28].copy_from_slice(&0u32.to_be_bytes());
    bytes[28..32].copy_from_slice(&0u32.to_be_bytes());
    bytes[32] = 5 << 4;
    bytes[33] = (packet.flags & 0xff) as u8;
    bytes[34..36].copy_from_slice(&8192u16.to_be_bytes());
    bytes[36..38].copy_from_slice(&0u16.to_be_bytes());
    bytes[38..40].copy_from_slice(&0u16.to_be_bytes());
    bytes[40..].copy_from_slice(&packet.payload);
    Ok(bytes)
}

pub fn udp_observation_from_virtual_packet(packet: &VirtualUdpPacket) -> UdpForwardObservation {
    UdpForwardObservation {
        source: SocketAddr::V4(SocketAddrV4::new(packet.source_ip, packet.source_port)),
        destination: SocketAddr::V4(SocketAddrV4::new(
            packet.destination_ip,
            packet.destination_port,
        )),
        bytes: packet.payload.len(),
        broadcast: packet.broadcast,
        direction: "virtual-adapter".to_owned(),
    }
}

pub fn tcp_observation_from_virtual_packet(packet: &VirtualTcpPacket) -> UdpForwardObservation {
    UdpForwardObservation {
        source: SocketAddr::V4(SocketAddrV4::new(packet.source_ip, packet.source_port)),
        destination: SocketAddr::V4(SocketAddrV4::new(
            packet.destination_ip,
            packet.destination_port,
        )),
        bytes: packet.payload.len(),
        broadcast: false,
        direction: "virtual-adapter".to_owned(),
    }
}

fn is_broadcast_ip(address: Ipv4Addr) -> bool {
    address == Ipv4Addr::BROADCAST || address.octets()[3] == 255
}

struct ParsedIpv4Header {
    header_len: usize,
    total_len: usize,
    protocol_number: u8,
    source_ip: Ipv4Addr,
    destination_ip: Ipv4Addr,
}

fn parse_ipv4_header(bytes: &[u8]) -> Result<ParsedIpv4Header, String> {
    if bytes.len() < 20 {
        return Err("IPv4 packet is too short.".to_owned());
    }
    let version = bytes[0] >> 4;
    let header_len = (bytes[0] & 0x0f) as usize * 4;
    if version != 4 {
        return Err("Packet is not IPv4.".to_owned());
    }
    if header_len < 20 || bytes.len() < header_len {
        return Err("IPv4 header length is invalid.".to_owned());
    }
    let total_len = u16::from_be_bytes([bytes[2], bytes[3]]) as usize;
    if total_len < header_len || total_len > bytes.len() {
        return Err("IPv4 total length is invalid.".to_owned());
    }

    Ok(ParsedIpv4Header {
        header_len,
        total_len,
        protocol_number: bytes[9],
        source_ip: Ipv4Addr::new(bytes[12], bytes[13], bytes[14], bytes[15]),
        destination_ip: Ipv4Addr::new(bytes[16], bytes[17], bytes[18], bytes[19]),
    })
}

fn write_ipv4_header(
    bytes: &mut [u8],
    total_len: usize,
    ttl: u8,
    protocol_number: u8,
    source_ip: Ipv4Addr,
    destination_ip: Ipv4Addr,
) {
    bytes[0] = 0x45;
    bytes[1] = 0;
    bytes[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
    bytes[4..6].copy_from_slice(&0u16.to_be_bytes());
    bytes[6..8].copy_from_slice(&0u16.to_be_bytes());
    bytes[8] = ttl;
    bytes[9] = protocol_number;
    bytes[12..16].copy_from_slice(&source_ip.octets());
    bytes[16..20].copy_from_slice(&destination_ip.octets());
    let checksum = ipv4_header_checksum(&bytes[..20]);
    bytes[10..12].copy_from_slice(&checksum.to_be_bytes());
}

fn ipv4_header_checksum(header: &[u8]) -> u16 {
    let mut sum = 0u32;
    for chunk in header.chunks_exact(2) {
        sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_udp_packet_round_trips() {
        let packet = VirtualUdpPacket {
            source_ip: "10.77.12.2".parse().unwrap(),
            destination_ip: "10.77.12.255".parse().unwrap(),
            source_port: 39077,
            destination_port: 27015,
            payload: b"discover".to_vec(),
            broadcast: true,
        };

        let bytes = build_ipv4_udp_packet(&packet, 64).unwrap();
        let parsed = parse_ipv4_udp_packet(&bytes).unwrap();

        assert_eq!(parsed, packet);
        assert!(parsed.broadcast);
    }

    #[test]
    fn ipv4_udp_packet_rejects_non_udp() {
        let mut bytes = build_ipv4_udp_packet(
            &VirtualUdpPacket {
                source_ip: "10.77.12.2".parse().unwrap(),
                destination_ip: "10.77.12.3".parse().unwrap(),
                source_port: 39077,
                destination_port: 27015,
                payload: b"hello".to_vec(),
                broadcast: false,
            },
            64,
        )
        .unwrap();
        bytes[9] = 6;

        let err = parse_ipv4_udp_packet(&bytes).unwrap_err();

        assert!(err.contains("not UDP"));
    }

    #[test]
    fn ipv4_tcp_packet_round_trips() {
        let packet = VirtualTcpPacket {
            source_ip: "10.77.12.2".parse().unwrap(),
            destination_ip: "10.77.12.3".parse().unwrap(),
            source_port: 50123,
            destination_port: 27015,
            payload: b"tcp hello".to_vec(),
            flags: 0x18,
        };

        let bytes = build_ipv4_tcp_packet(&packet, 64).unwrap();
        let parsed = parse_ipv4_tcp_packet(&bytes).unwrap();
        let summary = parse_ipv4_packet_summary(&bytes).unwrap();

        assert_eq!(parsed, packet);
        assert_eq!(summary.protocol, "tcp");
        assert_eq!(summary.destination_port, Some(27015));
        assert_eq!(summary.payload_bytes, 9);
    }

    #[test]
    fn ipv4_summary_reports_icmp_without_ports() {
        let mut bytes = vec![0u8; 28];
        write_ipv4_header(
            &mut bytes,
            28,
            64,
            1,
            "10.77.12.2".parse().unwrap(),
            "10.77.12.3".parse().unwrap(),
        );
        bytes[20] = 8;

        let summary = parse_ipv4_packet_summary(&bytes).unwrap();

        assert_eq!(summary.protocol, "icmp");
        assert_eq!(summary.source_port, None);
        assert_eq!(summary.destination_port, None);
        assert_eq!(summary.payload_bytes, 8);
    }

    #[test]
    fn virtual_packet_io_plan_marks_raw_adapter_backends() {
        let plan = create_virtual_packet_io_plan("LocalAreaInterconnection", "wintun", 1420);

        assert!(plan.can_read_ipv4);
        assert!(plan.can_write_ipv4);
        assert!(plan.can_observe_udp);
    }
}
