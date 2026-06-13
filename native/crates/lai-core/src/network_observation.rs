use crate::diagnostics::{evaluate_diagnostics, DiagnosticReport, DiagnosticSnapshot};
use crate::ip::Ipv4Subnet;
use crate::windows_route_parser::WindowsRouteObservation;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct NetworkObservationSnapshot {
    pub adapter: Option<AdapterObservation>,
    pub tunnel: Option<TunnelObservation>,
    pub packets: Vec<PacketObservation>,
    pub expected_peer_count: u16,
    pub expected_broadcast_ports: Vec<u16>,
    pub expected_game_ports: Vec<u16>,
    #[serde(default)]
    pub route_observations: Vec<WindowsRouteObservation>,
    #[serde(default)]
    pub runtime_peers: Vec<RuntimePeerObservation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdapterObservation {
    pub adapter_name: String,
    pub enabled: bool,
    pub expected_ip: Option<Ipv4Addr>,
    pub assigned_ip: Option<Ipv4Addr>,
    pub virtual_subnet: Option<Ipv4Subnet>,
    pub mtu: Option<u16>,
    pub interface_metric: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TunnelObservation {
    pub state: String,
    pub connected_peer_count: u16,
    pub latency_ms: Option<u32>,
    pub packet_loss_percent: Option<f32>,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PacketObservation {
    pub protocol: String,
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
    pub destination_port: u16,
    pub bytes: u32,
    pub direction: String,
    pub broadcast: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePeerObservation {
    pub peer_id: String,
    pub virtual_ip: String,
    pub selected_path: String,
    pub connection_path_status: String,
    pub bootstrap_status: String,
    pub connected: bool,
    pub path_kind: Option<String>,
    pub latency_ms: Option<u64>,
    pub last_seen_at_ms: Option<u64>,
    pub last_sent_at_ms: Option<u64>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub direct_bytes_sent: u64,
    pub direct_bytes_received: u64,
    pub relay_bytes_sent: u64,
    pub relay_bytes_received: u64,
    pub unknown_path_bytes_sent: u64,
    pub unknown_path_bytes_received: u64,
    pub heartbeat_packets_sent: u64,
    pub heartbeat_ack_packets_received: u64,
    pub heartbeat_loss_percent: Option<f64>,
    pub heartbeat_loss_window_size: usize,
    pub heartbeat_loss_window_percent: Option<f64>,
    pub heartbeat_rtt_sample_count: usize,
    pub heartbeat_rtt_jitter_ms: Option<f64>,
    pub forwarded_packets_sent: u64,
    pub tunnel_packets_received: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetworkObservationReport {
    pub status: String,
    pub summary: String,
    pub diagnostic_snapshot: DiagnosticSnapshot,
    pub diagnostic_report: DiagnosticReport,
    pub checks: Vec<NetworkObservationCheck>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetworkObservationCheck {
    pub key: String,
    pub status: String,
    pub message: String,
    pub next_action: String,
}

pub fn evaluate_network_observations(
    snapshot: NetworkObservationSnapshot,
) -> NetworkObservationReport {
    let checks = create_checks(&snapshot);
    let diagnostic_snapshot = DiagnosticSnapshot {
        virtual_adapter: Some(adapter_status(snapshot.adapter.as_ref()).to_owned()),
        tunnel: Some(tunnel_status(snapshot.tunnel.as_ref()).to_owned()),
        p2p: Some(p2p_status(snapshot.tunnel.as_ref(), snapshot.expected_peer_count).to_owned()),
        broadcast: Some(broadcast_status(&snapshot).to_owned()),
        game_traffic: Some(game_traffic_status(&snapshot).to_owned()),
        ..DiagnosticSnapshot::default()
    };
    let diagnostic_report = evaluate_diagnostics(diagnostic_snapshot.clone());
    let problem_count = checks
        .iter()
        .filter(|check| !matches!(check.status.as_str(), "ok" | "skipped"))
        .count();
    let status = if problem_count == 0 && diagnostic_report.problems.is_empty() {
        "ok"
    } else {
        "needs-attention"
    }
    .to_owned();

    NetworkObservationReport {
        status: status.clone(),
        summary: if status == "ok" {
            "Network experiment observations look healthy.".to_owned()
        } else {
            format!(
                "Detected {} network observation problem(s).",
                problem_count + diagnostic_report.problems.len()
            )
        },
        diagnostic_snapshot,
        diagnostic_report,
        checks,
    }
}

fn create_checks(snapshot: &NetworkObservationSnapshot) -> Vec<NetworkObservationCheck> {
    let mut checks = vec![
        check(
            "adapter",
            adapter_status(snapshot.adapter.as_ref()),
            "Virtual adapter observation is healthy.",
            "Inspect virtual adapter installation, enabled state, and assigned room IP.",
        ),
        check(
            "tunnel",
            tunnel_status(snapshot.tunnel.as_ref()),
            "Tunnel observation is healthy.",
            "Reconnect the tunnel, switch networks, or retry coordination.",
        ),
        check(
            "p2p",
            p2p_status(snapshot.tunnel.as_ref(), snapshot.expected_peer_count),
            "Expected peers are connected.",
            "Run NAT diagnostics and try port forwarding, network switching, or relay fallback.",
        ),
        check(
            "connection-path",
            connection_path_status(snapshot.tunnel.as_ref()),
            "Connection path is direct P2P.",
            "Review connection path diagnostics and use relay only when direct P2P is unavailable.",
        ),
        check(
            "broadcast",
            broadcast_status(snapshot),
            "Broadcast packets were observed.",
            "Check broadcast proxy rules and the game discovery port.",
        ),
        check(
            "game-traffic",
            game_traffic_status(snapshot),
            "Game traffic packets were observed.",
            "Check whether the game is using the virtual adapter and expected ports.",
        ),
        check(
            "route",
            route_status(snapshot),
            "A route covering the room virtual IP or subnet was observed.",
            "Check Windows route print output and ensure the room subnet routes through the virtual adapter.",
        ),
    ];
    checks.extend(runtime_peer_checks(&snapshot.runtime_peers));
    checks
}

pub fn runtime_peer_checks(peers: &[RuntimePeerObservation]) -> Vec<NetworkObservationCheck> {
    peers
        .iter()
        .map(|peer| {
            let health = runtime_peer_health(peer);
            NetworkObservationCheck {
                key: format!("runtime-peer:{}", peer.peer_id),
                status: health.status,
                message: format!("Runtime peer {} is {}.", peer.peer_id, health.reason),
                next_action: health.next_action,
            }
        })
        .collect()
}

fn adapter_status(adapter: Option<&AdapterObservation>) -> &'static str {
    let Some(adapter) = adapter else {
        return "missing";
    };
    if !adapter.enabled {
        return "disabled";
    }
    if let Some(expected_ip) = adapter.expected_ip {
        if adapter.assigned_ip != Some(expected_ip) {
            return "ip-mismatch";
        }
    }
    if let (Some(subnet), Some(assigned_ip)) = (adapter.virtual_subnet, adapter.assigned_ip) {
        if !subnet.contains(assigned_ip) {
            return "ip-outside-subnet";
        }
    }
    "ok"
}

fn tunnel_status(tunnel: Option<&TunnelObservation>) -> &'static str {
    let Some(tunnel) = tunnel else {
        return "missing";
    };
    if !tunnel.state.eq_ignore_ascii_case("connected") {
        return "down";
    }
    if tunnel.packet_loss_percent.unwrap_or(0.0) > 10.0 {
        return "high-loss";
    }
    "ok"
}

fn p2p_status(tunnel: Option<&TunnelObservation>, expected_peer_count: u16) -> &'static str {
    let Some(tunnel) = tunnel else {
        return "missing";
    };
    if !tunnel.state.eq_ignore_ascii_case("connected") {
        return "failed";
    }
    if expected_peer_count > 0 && tunnel.connected_peer_count < expected_peer_count {
        return "missing-peers";
    }
    "ok"
}

fn connection_path_status(tunnel: Option<&TunnelObservation>) -> &'static str {
    let Some(tunnel) = tunnel else {
        return "skipped";
    };
    if !tunnel.state.eq_ignore_ascii_case("connected") {
        return "skipped";
    }
    let Some(path) = tunnel.path.as_deref().map(str::trim) else {
        return "skipped";
    };
    if path.is_empty() {
        return "skipped";
    }
    if path.eq_ignore_ascii_case("p2p") || path.eq_ignore_ascii_case("direct") {
        "ok"
    } else if path.eq_ignore_ascii_case("relay") || path.eq_ignore_ascii_case("relayed") {
        "relay"
    } else if path.eq_ignore_ascii_case("none") || path.eq_ignore_ascii_case("failed") {
        "failed"
    } else {
        "unknown"
    }
}

fn broadcast_status(snapshot: &NetworkObservationSnapshot) -> &'static str {
    if packet_count(snapshot, true, &snapshot.expected_broadcast_ports) > 0 {
        "seen"
    } else {
        "missing"
    }
}

fn game_traffic_status(snapshot: &NetworkObservationSnapshot) -> &'static str {
    if packet_count(snapshot, false, &snapshot.expected_game_ports) > 0 {
        "seen"
    } else {
        "missing"
    }
}

fn route_status(snapshot: &NetworkObservationSnapshot) -> &'static str {
    if snapshot.route_observations.is_empty() {
        return "skipped";
    }
    let Some(adapter) = snapshot.adapter.as_ref() else {
        return "skipped";
    };
    let room_route_count = snapshot
        .route_observations
        .iter()
        .filter(|route| route.destination.prefix > 0)
        .filter(|route| {
            adapter
                .expected_ip
                .or(adapter.assigned_ip)
                .is_some_and(|ip| route.destination.contains(ip))
                || adapter
                    .virtual_subnet
                    .is_some_and(|subnet| route.destination.intersects(subnet))
        })
        .count();
    if room_route_count > 0 {
        "ok"
    } else {
        "missing-room-route"
    }
}

struct RuntimePeerHealth {
    status: String,
    reason: String,
    next_action: String,
}

fn runtime_peer_health(peer: &RuntimePeerObservation) -> RuntimePeerHealth {
    let (status, reason, next_action) = if matches!(
        peer.connection_path_status.as_str(),
        "no-path" | "needs-relay" | "config-error"
    ) || matches!(
        peer.selected_path.as_str(),
        "none" | "failed"
    ) {
        (
            "needs-attention",
            "no usable path",
            "Refresh NAT candidates or configure relay before starting the game.",
        )
    } else if !peer.connected && peer.heartbeat_ack_packets_received == 0 {
        (
            "needs-attention",
            "missing runtime packets",
            "Check that the peer runtime is still running and reachable on its tunnel endpoint.",
        )
    } else if peer
        .heartbeat_loss_window_percent
        .is_some_and(|loss| loss >= 50.0)
    {
        (
            "needs-attention",
            "high recent heartbeat loss",
            "Check firewall, NAT mapping, or relay fallback; recent heartbeat acknowledgements are missing.",
        )
    } else if peer.heartbeat_loss_percent.is_some_and(|loss| loss >= 50.0) {
        (
            "needs-attention",
            "high heartbeat loss",
            "Check firewall, NAT mapping, or relay fallback; heartbeat acknowledgements are missing.",
        )
    } else if peer.latency_ms.is_some_and(|latency| latency >= 150) {
        (
            "degraded",
            "high latency",
            "Direct IP may work, but expect delay; consider relay region or network changes.",
        )
    } else if peer
        .heartbeat_rtt_jitter_ms
        .is_some_and(|jitter| jitter >= 50.0)
    {
        (
            "degraded",
            "high jitter",
            "Direct IP may work, but unstable latency can affect games; consider relay region or network changes.",
        )
    } else {
        (
            "ok",
            "healthy",
            "Peer runtime path, heartbeat, and traffic evidence look healthy.",
        )
    };
    RuntimePeerHealth {
        status: status.to_owned(),
        reason: reason.to_owned(),
        next_action: next_action.to_owned(),
    }
}

fn packet_count(
    snapshot: &NetworkObservationSnapshot,
    broadcast: bool,
    expected_ports: &[u16],
) -> usize {
    snapshot
        .packets
        .iter()
        .filter(|packet| packet.broadcast == broadcast)
        .filter(|packet| {
            packet.protocol.eq_ignore_ascii_case("udp")
                || packet.protocol.eq_ignore_ascii_case("tcp")
        })
        .filter(|packet| {
            expected_ports.is_empty() || expected_ports.contains(&packet.destination_port)
        })
        .count()
}

fn check(
    key: &str,
    status: &str,
    healthy_message: &str,
    next_action: &str,
) -> NetworkObservationCheck {
    NetworkObservationCheck {
        key: key.to_owned(),
        status: if matches!(status, "ok" | "seen") {
            "ok".to_owned()
        } else if status == "skipped" {
            "skipped".to_owned()
        } else {
            status.to_owned()
        },
        message: if matches!(status, "ok" | "seen") {
            healthy_message.to_owned()
        } else if status == "skipped" {
            format!("{key} observation was skipped.")
        } else {
            format!("{key} observation is {status}.")
        },
        next_action: if matches!(status, "ok" | "seen") {
            "No action needed.".to_owned()
        } else if status == "skipped" {
            match key {
                "connection-path" => {
                    "Provide tunnel path evidence or run connection path diagnostics when P2P behavior is unclear."
                }
                "route" => "Provide route print -4 output when route diagnosis is needed.",
                _ => "Provide more observation evidence when this diagnosis is needed.",
            }
            .to_owned()
        } else {
            next_action.to_owned()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(port: u16, broadcast: bool) -> PacketObservation {
        PacketObservation {
            protocol: "udp".to_owned(),
            source_ip: "10.77.12.2".parse().unwrap(),
            destination_ip: if broadcast {
                "10.77.12.255".parse().unwrap()
            } else {
                "10.77.12.3".parse().unwrap()
            },
            destination_port: port,
            bytes: 8,
            direction: "outbound".to_owned(),
            broadcast,
        }
    }

    #[test]
    fn network_observations_report_healthy_snapshot() {
        let report = evaluate_network_observations(NetworkObservationSnapshot {
            adapter: Some(AdapterObservation {
                adapter_name: "LocalAreaInterconnection".to_owned(),
                enabled: true,
                expected_ip: Some("10.77.12.2".parse().unwrap()),
                assigned_ip: Some("10.77.12.2".parse().unwrap()),
                virtual_subnet: Some("10.77.12.0/24".parse().unwrap()),
                mtu: Some(1420),
                interface_metric: Some(5),
            }),
            tunnel: Some(TunnelObservation {
                state: "connected".to_owned(),
                connected_peer_count: 1,
                latency_ms: Some(12),
                packet_loss_percent: Some(0.0),
                path: Some("p2p".to_owned()),
            }),
            packets: vec![packet(27015, true), packet(27015, false)],
            expected_peer_count: 1,
            expected_broadcast_ports: vec![27015],
            expected_game_ports: vec![27015],
            route_observations: Vec::new(),
            runtime_peers: Vec::new(),
        });

        assert_eq!(report.status, "ok");
        assert_eq!(
            report.diagnostic_snapshot.virtual_adapter.as_deref(),
            Some("ok")
        );
        assert_eq!(
            report.diagnostic_snapshot.broadcast.as_deref(),
            Some("seen")
        );
        assert!(report.diagnostic_report.problems.is_empty());
    }

    #[test]
    fn network_observations_report_missing_adapter_and_packets() {
        let report = evaluate_network_observations(NetworkObservationSnapshot {
            tunnel: Some(TunnelObservation {
                state: "disconnected".to_owned(),
                connected_peer_count: 0,
                latency_ms: None,
                packet_loss_percent: None,
                path: None,
            }),
            expected_peer_count: 1,
            expected_broadcast_ports: vec![27015],
            expected_game_ports: vec![27015],
            ..NetworkObservationSnapshot::default()
        });

        assert_eq!(report.status, "needs-attention");
        assert_eq!(
            report.diagnostic_snapshot.virtual_adapter.as_deref(),
            Some("missing")
        );
        assert_eq!(report.diagnostic_snapshot.p2p.as_deref(), Some("failed"));
        assert!(report
            .diagnostic_report
            .problems
            .iter()
            .any(|problem| problem.key == "virtual_adapter"));
    }

    #[test]
    fn network_observations_report_relay_connection_path_as_attention() {
        let report = evaluate_network_observations(NetworkObservationSnapshot {
            adapter: Some(AdapterObservation {
                adapter_name: "LocalAreaInterconnection".to_owned(),
                enabled: true,
                expected_ip: Some("10.77.12.2".parse().unwrap()),
                assigned_ip: Some("10.77.12.2".parse().unwrap()),
                virtual_subnet: Some("10.77.12.0/24".parse().unwrap()),
                mtu: Some(1420),
                interface_metric: Some(5),
            }),
            tunnel: Some(TunnelObservation {
                state: "connected".to_owned(),
                connected_peer_count: 1,
                latency_ms: Some(12),
                packet_loss_percent: Some(0.0),
                path: Some("relay".to_owned()),
            }),
            packets: vec![packet(27015, true), packet(27015, false)],
            expected_peer_count: 1,
            expected_broadcast_ports: vec![27015],
            expected_game_ports: vec![27015],
            route_observations: Vec::new(),
            runtime_peers: Vec::new(),
        });

        assert_eq!(report.status, "needs-attention");
        assert!(report
            .checks
            .iter()
            .any(|check| check.key == "connection-path" && check.status == "relay"));
    }

    #[test]
    fn network_observations_report_room_route_present() {
        let report = evaluate_network_observations(NetworkObservationSnapshot {
            adapter: Some(AdapterObservation {
                adapter_name: "LocalAreaInterconnection".to_owned(),
                enabled: true,
                expected_ip: Some("10.77.12.2".parse().unwrap()),
                assigned_ip: Some("10.77.12.2".parse().unwrap()),
                virtual_subnet: Some("10.77.12.0/24".parse().unwrap()),
                mtu: Some(1420),
                interface_metric: Some(5),
            }),
            route_observations: vec![WindowsRouteObservation {
                destination: "10.77.12.0/24".parse().unwrap(),
                gateway: None,
                interface_ip: Some("10.77.12.2".parse().unwrap()),
                metric: Some(5),
                persistent: false,
            }],
            tunnel: Some(TunnelObservation {
                state: "connected".to_owned(),
                connected_peer_count: 1,
                latency_ms: Some(12),
                packet_loss_percent: Some(0.0),
                path: Some("p2p".to_owned()),
            }),
            packets: vec![packet(27015, true), packet(27015, false)],
            expected_peer_count: 1,
            expected_broadcast_ports: vec![27015],
            expected_game_ports: vec![27015],
            runtime_peers: Vec::new(),
        });

        assert_eq!(report.status, "ok");
        assert!(report
            .checks
            .iter()
            .any(|check| check.key == "route" && check.status == "ok"));
    }

    #[test]
    fn network_observations_report_missing_room_route() {
        let report = evaluate_network_observations(NetworkObservationSnapshot {
            adapter: Some(AdapterObservation {
                adapter_name: "LocalAreaInterconnection".to_owned(),
                enabled: true,
                expected_ip: Some("10.77.12.2".parse().unwrap()),
                assigned_ip: Some("10.77.12.2".parse().unwrap()),
                virtual_subnet: Some("10.77.12.0/24".parse().unwrap()),
                mtu: Some(1420),
                interface_metric: Some(5),
            }),
            tunnel: Some(TunnelObservation {
                state: "connected".to_owned(),
                connected_peer_count: 1,
                latency_ms: Some(12),
                packet_loss_percent: Some(0.0),
                path: Some("p2p".to_owned()),
            }),
            packets: vec![packet(27015, true), packet(27015, false)],
            expected_peer_count: 1,
            expected_broadcast_ports: vec![27015],
            expected_game_ports: vec![27015],
            route_observations: vec![WindowsRouteObservation {
                destination: "192.168.1.0/24".parse().unwrap(),
                gateway: Some("192.168.1.1".parse().unwrap()),
                interface_ip: Some("192.168.1.10".parse().unwrap()),
                metric: Some(25),
                persistent: false,
            }],
            runtime_peers: Vec::new(),
        });

        assert_eq!(report.status, "needs-attention");
        assert!(report
            .checks
            .iter()
            .any(|check| check.key == "route" && check.status == "missing-room-route"));
    }

    #[test]
    fn network_observations_include_runtime_peer_checks() {
        let report = evaluate_network_observations(NetworkObservationSnapshot {
            adapter: None,
            tunnel: None,
            packets: Vec::new(),
            expected_peer_count: 1,
            expected_broadcast_ports: Vec::new(),
            expected_game_ports: Vec::new(),
            route_observations: Vec::new(),
            runtime_peers: vec![RuntimePeerObservation {
                peer_id: "peer_b".to_owned(),
                virtual_ip: "10.77.12.3".to_owned(),
                selected_path: "failed".to_owned(),
                connection_path_status: "no-path".to_owned(),
                bootstrap_status: "failed".to_owned(),
                connected: false,
                ..RuntimePeerObservation::default()
            }],
        });

        assert_eq!(report.status, "needs-attention");
        assert!(report.checks.iter().any(|check| {
            check.key == "runtime-peer:peer_b" && check.status == "needs-attention"
        }));
    }
}
