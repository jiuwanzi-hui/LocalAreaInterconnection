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
pub struct BroadcastForwardEvent {
    pub protocol: String,
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
    pub destination_port: u16,
    pub forwarded: bool,
    pub reason: String,
    pub target_count: usize,
    pub packet_io_backend: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BroadcastForwardReport {
    pub status: String,
    pub event_count: usize,
    pub forwarded_event_count: usize,
    pub dropped_event_count: usize,
    pub forwarded_target_count: usize,
    pub rate_limited_count: usize,
    pub allowed_ports: Vec<u16>,
    pub max_packets_per_second: u16,
    pub events: Vec<BroadcastForwardEvent>,
    pub next_action: String,
}

#[derive(Clone, Debug)]
pub struct BroadcastForwardGate {
    policy: BroadcastPolicy,
    window_started_ms: u128,
    forwarded_in_window: u16,
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

impl BroadcastForwardGate {
    pub fn new(policy: BroadcastPolicy, now_ms: u128) -> Self {
        Self {
            policy,
            window_started_ms: now_ms,
            forwarded_in_window: 0,
        }
    }

    pub fn policy(&self) -> &BroadcastPolicy {
        &self.policy
    }

    pub fn decide(&mut self, packet: &BroadcastPacket, now_ms: u128) -> BroadcastDecision {
        if now_ms.saturating_sub(self.window_started_ms) >= 1000 {
            self.window_started_ms = now_ms;
            self.forwarded_in_window = 0;
        }
        let decision = should_forward_broadcast(packet, &self.policy);
        if !decision.forward {
            return decision;
        }
        if self.policy.max_packets_per_second > 0
            && self.forwarded_in_window >= self.policy.max_packets_per_second
        {
            return deny("broadcast-rate-limited");
        }
        self.forwarded_in_window = self.forwarded_in_window.saturating_add(1);
        decision
    }
}

pub fn create_broadcast_forward_report(
    policy: &BroadcastPolicy,
    events: Vec<BroadcastForwardEvent>,
) -> BroadcastForwardReport {
    let forwarded_event_count = events.iter().filter(|event| event.forwarded).count();
    let dropped_event_count = events.len().saturating_sub(forwarded_event_count);
    let forwarded_target_count = events
        .iter()
        .filter(|event| event.forwarded)
        .map(|event| event.target_count)
        .sum::<usize>();
    let rate_limited_count = events
        .iter()
        .filter(|event| event.reason == "broadcast-rate-limited")
        .count();
    let status = if events.is_empty() {
        "idle"
    } else if dropped_event_count == 0 {
        "ok"
    } else if forwarded_event_count > 0 {
        "partial"
    } else {
        "blocked"
    }
    .to_owned();
    let next_action = match status.as_str() {
        "idle" => {
            "Start the game LAN browser or run a broadcast test to produce discovery packets."
        }
        "ok" => "Broadcast forwarding decisions look healthy.",
        "partial" => {
            "Review dropped broadcast events; check allowed ports, room subnet, and rate limits."
        }
        _ => {
            "No broadcast packets were forwarded; check the game discovery port and virtual subnet."
        }
    }
    .to_owned();

    BroadcastForwardReport {
        status,
        event_count: events.len(),
        forwarded_event_count,
        dropped_event_count,
        forwarded_target_count,
        rate_limited_count,
        allowed_ports: policy.allowed_ports.clone(),
        max_packets_per_second: policy.max_packets_per_second,
        events,
        next_action,
    }
}

fn deny(reason: &'static str) -> BroadcastDecision {
    BroadcastDecision {
        forward: false,
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> BroadcastPolicy {
        BroadcastPolicy::with_limit("10.77.12.0/24".parse().unwrap(), vec![27015], 1)
    }

    #[test]
    fn gate_allows_room_broadcast_then_rate_limits() {
        let mut gate = BroadcastForwardGate::new(policy(), 1000);
        let packet = BroadcastPacket {
            protocol: "udp".to_owned(),
            source_ip: "10.77.12.2".parse().unwrap(),
            destination_ip: "10.77.12.255".parse().unwrap(),
            destination_port: 27015,
        };

        assert_eq!(gate.decide(&packet, 1000).reason, "room-broadcast-allowed");
        assert_eq!(gate.decide(&packet, 1001).reason, "broadcast-rate-limited");
        assert_eq!(gate.decide(&packet, 2001).reason, "room-broadcast-allowed");
    }

    #[test]
    fn report_summarizes_forward_and_drop_events() {
        let policy = policy();
        let report = create_broadcast_forward_report(
            &policy,
            vec![
                BroadcastForwardEvent {
                    protocol: "udp".to_owned(),
                    source_ip: "10.77.12.2".parse().unwrap(),
                    destination_ip: "10.77.12.255".parse().unwrap(),
                    destination_port: 27015,
                    forwarded: true,
                    reason: "room-broadcast-allowed".to_owned(),
                    target_count: 2,
                    packet_io_backend: "wintun".to_owned(),
                },
                BroadcastForwardEvent {
                    protocol: "udp".to_owned(),
                    source_ip: "10.77.12.2".parse().unwrap(),
                    destination_ip: "10.77.12.255".parse().unwrap(),
                    destination_port: 27016,
                    forwarded: false,
                    reason: "destination-port-not-allowed".to_owned(),
                    target_count: 0,
                    packet_io_backend: "wintun".to_owned(),
                },
            ],
        );

        assert_eq!(report.status, "partial");
        assert_eq!(report.forwarded_event_count, 1);
        assert_eq!(report.dropped_event_count, 1);
        assert_eq!(report.forwarded_target_count, 2);
    }
}
