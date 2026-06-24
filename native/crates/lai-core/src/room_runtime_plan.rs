use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoomRuntimePeer {
    pub peer_id: String,
    pub virtual_ip: Ipv4Addr,
    pub endpoint: String,
    #[serde(default = "default_runtime_peer_path")]
    pub connection_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direct_endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_endpoint: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeTunnelPlan {
    pub bind_endpoint: String,
    pub encryption: String,
    pub handshake: String,
    pub peer_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimePortBinding {
    pub protocol: String,
    pub port: u16,
    pub purpose: String,
    pub observe_packets: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeUdpForwardPlan {
    pub listen_endpoint: String,
    pub forward_to_peers: Vec<String>,
    pub port: u16,
    pub broadcast: bool,
    pub rate_limit_packets_per_second: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoomRuntimePlan {
    pub room_id: String,
    pub local_peer_id: String,
    pub local_virtual_ip: Ipv4Addr,
    pub tunnel: RuntimeTunnelPlan,
    pub peers: Vec<RoomRuntimePeer>,
    pub capture_ports: Vec<RuntimePortBinding>,
    pub udp_forwarders: Vec<RuntimeUdpForwardPlan>,
    pub diagnostic_outputs: Vec<String>,
    pub warnings: Vec<String>,
}

fn default_runtime_peer_path() -> String {
    "direct".to_owned()
}

pub fn create_room_runtime_plan(
    room_id: impl Into<String>,
    local_peer_id: impl Into<String>,
    local_virtual_ip: Ipv4Addr,
    bind_endpoint: impl Into<String>,
    peers: Vec<RoomRuntimePeer>,
    game_ports: Vec<u16>,
    broadcast_ports: Vec<u16>,
) -> RoomRuntimePlan {
    let bind_endpoint = bind_endpoint.into();
    let mut capture_ports = Vec::new();
    for port in &game_ports {
        capture_ports.push(RuntimePortBinding {
            protocol: "udp".to_owned(),
            port: *port,
            purpose: "game-traffic".to_owned(),
            observe_packets: true,
        });
    }
    for port in &broadcast_ports {
        if !game_ports.contains(port) || *port == 0 {
            capture_ports.push(RuntimePortBinding {
                protocol: "udp".to_owned(),
                port: *port,
                purpose: "broadcast-discovery".to_owned(),
                observe_packets: true,
            });
        }
    }

    let peer_endpoints = peers
        .iter()
        .map(|peer| peer.endpoint.clone())
        .collect::<Vec<_>>();
    let udp_forwarders = broadcast_ports
        .iter()
        .map(|port| RuntimeUdpForwardPlan {
            listen_endpoint: format!("0.0.0.0:{port}"),
            forward_to_peers: peer_endpoints.clone(),
            port: *port,
            broadcast: true,
            rate_limit_packets_per_second: 30,
        })
        .collect::<Vec<_>>();

    let mut warnings = Vec::new();
    if peers.is_empty() {
        warnings.push("No peer endpoints are configured; P2P cannot connect yet.".to_owned());
    }
    if game_ports.is_empty() && broadcast_ports.is_empty() {
        warnings
            .push("No game or broadcast ports are configured for packet observation.".to_owned());
    }

    RoomRuntimePlan {
        room_id: room_id.into(),
        local_peer_id: local_peer_id.into(),
        local_virtual_ip,
        tunnel: RuntimeTunnelPlan {
            bind_endpoint,
            encryption: "chacha20poly1305-sha256-key".to_owned(),
            handshake: "encrypted-p2p-hello-ack".to_owned(),
            peer_count: peers.len(),
        },
        peers,
        capture_ports,
        udp_forwarders,
        diagnostic_outputs: vec![
            "TunnelServiceSnapshot".to_owned(),
            "PacketCaptureSummary".to_owned(),
            "packet-observation-lines".to_owned(),
        ],
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_runtime_plan_combines_tunnel_capture_and_forwarding() {
        let plan = create_room_runtime_plan(
            "room_1",
            "peer_a",
            "10.77.12.2".parse().unwrap(),
            "0.0.0.0:39090",
            vec![RoomRuntimePeer {
                peer_id: "peer_b".to_owned(),
                virtual_ip: "10.77.12.3".parse().unwrap(),
                endpoint: "203.0.113.10:39090".to_owned(),
                connection_path: "direct".to_owned(),
                direct_endpoint: Some("203.0.113.10:39090".to_owned()),
                fallback_endpoint: None,
            }],
            vec![27015],
            vec![27015, 39078],
        );

        assert_eq!(plan.tunnel.peer_count, 1);
        assert_eq!(plan.capture_ports.len(), 2);
        assert_eq!(plan.udp_forwarders.len(), 2);
        assert!(plan.warnings.is_empty());
        assert_eq!(
            plan.udp_forwarders[0].forward_to_peers[0],
            "203.0.113.10:39090"
        );
    }
}
