use crate::ip::{broadcast_address, Ipv4Subnet};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BroadcastPolicy {
    pub virtual_subnet: Ipv4SubnetText,
    pub subnet_broadcast: Ipv4Addr,
    pub allowed_ports: Vec<u16>,
    pub max_packets_per_second: u16,
    pub forward_global_broadcast: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BroadcastPacket {
    pub protocol: String,
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
    pub destination_port: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BroadcastDecision {
    pub forward: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Ipv4SubnetText(pub String);

impl BroadcastPolicy {
    pub fn new(virtual_subnet: Ipv4Subnet, allowed_ports: Vec<u16>) -> Self {
        Self::with_limit(virtual_subnet, allowed_ports, 30)
    }

    pub fn with_limit(
        virtual_subnet: Ipv4Subnet,
        allowed_ports: Vec<u16>,
        max_packets_per_second: u16,
    ) -> Self {
        Self {
            virtual_subnet: Ipv4SubnetText(virtual_subnet.to_string()),
            subnet_broadcast: broadcast_address(virtual_subnet),
            allowed_ports,
            max_packets_per_second,
            forward_global_broadcast: true,
        }
    }
}

pub fn should_forward_broadcast(
    packet: &BroadcastPacket,
    policy: &BroadcastPolicy,
) -> BroadcastDecision {
    let subnet = match policy.virtual_subnet.0.parse::<Ipv4Subnet>() {
        Ok(value) => value,
        Err(_) => return deny("invalid-policy-subnet"),
    };
    if packet.protocol != "udp" {
        return deny("only-udp-broadcast-is-forwarded");
    }
    if !subnet.contains(packet.source_ip) {
        return deny("source-outside-room-subnet");
    }
    let global = packet.destination_ip == Ipv4Addr::new(255, 255, 255, 255);
    let room = packet.destination_ip == policy.subnet_broadcast;
    if !room && !(global && policy.forward_global_broadcast) {
        return deny("destination-is-not-room-broadcast");
    }
    if !policy.allowed_ports.is_empty() && !policy.allowed_ports.contains(&packet.destination_port)
    {
        return deny("destination-port-not-allowed");
    }
    BroadcastDecision {
        forward: true,
        reason: "room-broadcast-allowed".to_owned(),
    }
}

fn deny(reason: &'static str) -> BroadcastDecision {
    BroadcastDecision {
        forward: false,
        reason: reason.to_owned(),
    }
}
