use crate::connection_path::ConnectionPathReport;
use crate::firewall_diagnostics::FirewallDiagnosticsReport;
use crate::game_network_plan::GameNetworkPlan;
use crate::network_observation::NetworkObservationReport;
use crate::windows_netstat_parser::WindowsNetstatEndpoint;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameReadinessReport {
    pub status: String,
    pub summary: String,
    pub game_name: String,
    pub recommended_join: String,
    pub checks: Vec<GameReadinessCheck>,
    pub next_actions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GameReadinessCheck {
    pub key: String,
    pub status: String,
    pub message: String,
    pub next_action: String,
}

pub fn evaluate_game_readiness(
    plan: &GameNetworkPlan,
    network: &NetworkObservationReport,
    port_matches: &[WindowsNetstatEndpoint],
) -> GameReadinessReport {
    evaluate_game_readiness_with_firewall(plan, network, port_matches, None)
}

pub fn evaluate_game_readiness_with_firewall(
    plan: &GameNetworkPlan,
    network: &NetworkObservationReport,
    port_matches: &[WindowsNetstatEndpoint],
    firewall: Option<&FirewallDiagnosticsReport>,
) -> GameReadinessReport {
    evaluate_game_readiness_with_firewall_and_connection_path(
        plan,
        network,
        port_matches,
        firewall,
        None,
    )
}

pub fn evaluate_game_readiness_with_firewall_and_connection_path(
    plan: &GameNetworkPlan,
    network: &NetworkObservationReport,
    port_matches: &[WindowsNetstatEndpoint],
    firewall: Option<&FirewallDiagnosticsReport>,
    connection_path: Option<&ConnectionPathReport>,
) -> GameReadinessReport {
    let mut checks = vec![
        adapter_check(network),
        tunnel_check(network),
        p2p_check(network, connection_path),
        connection_path_check(connection_path),
        route_check(network),
        firewall_check(firewall),
        broadcast_check(plan, network),
        port_binding_check(plan, port_matches),
        game_traffic_check(network),
    ];
    checks.extend(runtime_peer_readiness_checks(network));
    let blockers = checks
        .iter()
        .filter(|check| {
            matches!(
                check.status.as_str(),
                "missing" | "failed" | "needs-attention"
            )
        })
        .count();
    let pending = checks
        .iter()
        .filter(|check| check.status == "pending")
        .count();
    let status = if blockers > 0 {
        "needs-attention"
    } else if pending > 0 {
        "ready-to-try"
    } else {
        "ready"
    }
    .to_owned();
    let next_actions = checks
        .iter()
        .filter(|check| check.status != "ok" && check.status != "skipped")
        .map(|check| check.next_action.clone())
        .collect::<Vec<_>>();

    GameReadinessReport {
        status: status.clone(),
        summary: match status.as_str() {
            "ready" => "Game readiness checks look ready for a LAN attempt.".to_owned(),
            "ready-to-try" => {
                "Network checks are usable; start or host the game and watch for traffic."
                    .to_owned()
            }
            _ => format!("Game readiness has {blockers} blocker(s) needing attention."),
        },
        game_name: plan.game_name.clone(),
        recommended_join: plan.join_instruction.clone(),
        checks,
        next_actions,
    }
}

fn firewall_check(firewall: Option<&FirewallDiagnosticsReport>) -> GameReadinessCheck {
    let Some(firewall) = firewall else {
        return readiness_check(
            "firewall",
            "skipped",
            "Firewall diagnostics were not provided.",
            "Run firewall diagnostics if the game cannot host or accept Direct IP connections.",
        );
    };
    let status = match firewall.status.as_str() {
        "ok" | "unknown" => firewall.status.as_str(),
        _ => "needs-attention",
    };
    readiness_check(
        "firewall",
        status,
        "Expected Windows Firewall rules are acceptable.",
        "Add or fix the expected inbound firewall rules for this game profile.",
    )
}

fn adapter_check(network: &NetworkObservationReport) -> GameReadinessCheck {
    let status = network_status(network, "adapter").unwrap_or("missing");
    readiness_check(
        "adapter",
        if status == "ok" { "ok" } else { "failed" },
        "Virtual adapter is ready.",
        "Run adapter ensure/apply and verify the room virtual IP.",
    )
}

fn tunnel_check(network: &NetworkObservationReport) -> GameReadinessCheck {
    let status = network_status(network, "tunnel").unwrap_or("missing");
    readiness_check(
        "tunnel",
        if status == "ok" { "ok" } else { "failed" },
        "Tunnel is connected.",
        "Reconnect the runtime, retry coordination, or inspect tunnel diagnostics.",
    )
}

fn p2p_check(
    network: &NetworkObservationReport,
    connection_path: Option<&ConnectionPathReport>,
) -> GameReadinessCheck {
    let status = network_status(network, "p2p").unwrap_or("missing");
    if status != "ok"
        && connection_path.is_some_and(|report| report.selected_path.eq_ignore_ascii_case("relay"))
    {
        return readiness_check(
            "p2p",
            "pending",
            "Direct P2P is unavailable, but a relay path is available.",
            "Start the relay fallback path before attempting LAN discovery or Direct IP.",
        );
    }
    readiness_check(
        "p2p",
        if status == "ok" { "ok" } else { "failed" },
        "Expected peers are connected.",
        "Run NAT diagnostics or try relay/port-forward fallback.",
    )
}

fn connection_path_check(connection_path: Option<&ConnectionPathReport>) -> GameReadinessCheck {
    let Some(report) = connection_path else {
        return readiness_check(
            "connection-path",
            "skipped",
            "Connection path diagnostics were not provided.",
            "Run connection path diagnostics if P2P fails or NAT behavior is unclear.",
        );
    };
    match report.selected_path.as_str() {
        "p2p" => readiness_check(
            "connection-path",
            "ok",
            "P2P connection path candidates are available.",
            "No action needed.",
        ),
        "relay" => readiness_check(
            "connection-path",
            "pending",
            "A relay path is available after P2P failure.",
            "Use relay fallback and expect higher latency than direct P2P.",
        ),
        _ => readiness_check(
            "connection-path",
            "failed",
            "No usable P2P or relay path is available.",
            "Refresh NAT candidates, change network, configure port forwarding, or provide a relay endpoint.",
        ),
    }
}

fn route_check(network: &NetworkObservationReport) -> GameReadinessCheck {
    let status = network_status(network, "route").unwrap_or("skipped");
    readiness_check(
        "route",
        if matches!(status, "ok" | "skipped") {
            status
        } else {
            "needs-attention"
        },
        "Room route evidence is acceptable.",
        "Scan Windows routes and ensure the room subnet routes through the virtual adapter.",
    )
}

fn broadcast_check(
    plan: &GameNetworkPlan,
    network: &NetworkObservationReport,
) -> GameReadinessCheck {
    if !plan.broadcast.enabled {
        return readiness_check(
            "broadcast",
            "skipped",
            "This game plan does not require LAN broadcast discovery.",
            "Use Direct IP or the profile's recommended join method.",
        );
    }
    let status = network_status(network, "broadcast").unwrap_or("missing");
    readiness_check(
        "broadcast",
        if status == "ok" { "ok" } else { "pending" },
        "Broadcast discovery packets have been observed.",
        "Open the game's LAN browser or run the broadcast test to confirm discovery packets.",
    )
}

fn port_binding_check(
    plan: &GameNetworkPlan,
    port_matches: &[WindowsNetstatEndpoint],
) -> GameReadinessCheck {
    if plan.firewall_rules.is_empty() {
        return readiness_check(
            "game-port-binding",
            "skipped",
            "No expected game ports are configured.",
            "Add a game profile or manual ports before port binding checks.",
        );
    }
    readiness_check(
        "game-port-binding",
        if port_matches.is_empty() {
            "pending"
        } else {
            "ok"
        },
        "Expected game ports are bound by a local process.",
        "Start or host the game, then run game port scan again.",
    )
}

fn game_traffic_check(network: &NetworkObservationReport) -> GameReadinessCheck {
    let status = network_status(network, "game-traffic").unwrap_or("missing");
    readiness_check(
        "game-traffic",
        if status == "ok" { "ok" } else { "pending" },
        "Game traffic has been observed on expected ports.",
        "Start the game and attempt LAN/Direct IP join, then rerun network diagnostics.",
    )
}

fn runtime_peer_readiness_checks(network: &NetworkObservationReport) -> Vec<GameReadinessCheck> {
    network
        .checks
        .iter()
        .filter(|check| check.key.starts_with("runtime-peer:"))
        .map(|check| {
            let status = match check.status.as_str() {
                "ok" | "skipped" => check.status.as_str(),
                "pending" | "degraded" => "pending",
                _ => "needs-attention",
            };
            readiness_check(&check.key, status, &check.message, &check.next_action)
        })
        .collect()
}

fn network_status<'a>(network: &'a NetworkObservationReport, key: &str) -> Option<&'a str> {
    network
        .checks
        .iter()
        .find(|check| check.key == key)
        .map(|check| check.status.as_str())
}

fn readiness_check(
    key: &str,
    status: &str,
    message: &str,
    next_action: &str,
) -> GameReadinessCheck {
    GameReadinessCheck {
        key: key.to_owned(),
        status: status.to_owned(),
        message: message.to_owned(),
        next_action: if status == "ok" {
            "No action needed.".to_owned()
        } else {
            next_action.to_owned()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_network_plan::create_game_network_plan;
    use crate::game_profile::{CompatibilityLevel, DiscoveryMode, GameProfile};
    use crate::network_observation::{
        evaluate_network_observations, AdapterObservation, NetworkObservationSnapshot,
        PacketObservation, RuntimePeerObservation, TunnelObservation,
    };
    use crate::{evaluate_connection_path, NatCandidate, NatTraversalOffer};

    fn plan() -> GameNetworkPlan {
        create_game_network_plan(
            &GameProfile {
                game_name: "Example".to_owned(),
                steam_app_id: None,
                discovery: DiscoveryMode::UdpBroadcast,
                ports: vec![27015],
                join_method: "lan_list_or_direct_ip".to_owned(),
                compatibility: CompatibilityLevel::A,
                notes: String::new(),
            },
            "10.77.12.0/24".parse().unwrap(),
            Some("10.77.12.1".parse().unwrap()),
            Some("10.77.12.2".parse().unwrap()),
            30,
        )
    }

    fn healthy_network() -> NetworkObservationReport {
        network_with_connected_peers(1)
    }

    fn network_with_connected_peers(connected_peer_count: u16) -> NetworkObservationReport {
        evaluate_network_observations(NetworkObservationSnapshot {
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
                connected_peer_count,
                latency_ms: Some(12),
                packet_loss_percent: Some(0.0),
                path: Some("p2p".to_owned()),
            }),
            packets: vec![
                PacketObservation {
                    protocol: "udp".to_owned(),
                    source_ip: "10.77.12.2".parse().unwrap(),
                    destination_ip: "10.77.12.255".parse().unwrap(),
                    destination_port: 27015,
                    bytes: 8,
                    direction: "outbound".to_owned(),
                    broadcast: true,
                },
                PacketObservation {
                    protocol: "udp".to_owned(),
                    source_ip: "10.77.12.2".parse().unwrap(),
                    destination_ip: "10.77.12.3".parse().unwrap(),
                    destination_port: 27015,
                    bytes: 8,
                    direction: "outbound".to_owned(),
                    broadcast: false,
                },
            ],
            expected_peer_count: 1,
            expected_broadcast_ports: vec![27015],
            expected_game_ports: vec![27015],
            route_observations: Vec::new(),
            runtime_peers: Vec::new(),
        })
    }

    fn offer(peer_id: &str, candidates: Vec<NatCandidate>) -> NatTraversalOffer {
        NatTraversalOffer {
            schema_version: 1,
            room_id: "room_test".to_owned(),
            peer_id: peer_id.to_owned(),
            virtual_ip: None,
            nonce: format!("nonce-{peer_id}"),
            created_at_ms: 1,
            candidates,
        }
    }

    fn candidate(candidate_type: &str, endpoint: &str, priority: u32) -> NatCandidate {
        NatCandidate {
            candidate_type: candidate_type.to_owned(),
            transport: "udp".to_owned(),
            endpoint: endpoint.to_owned(),
            priority,
            source: "test".to_owned(),
        }
    }

    #[test]
    fn readiness_is_ready_when_network_and_port_binding_are_present() {
        let report = evaluate_game_readiness(
            &plan(),
            &healthy_network(),
            &[WindowsNetstatEndpoint {
                protocol: "udp".to_owned(),
                local_address: None,
                local_port: Some(27015),
                foreign_address: None,
                foreign_port: None,
                state: None,
                pid: Some(4242),
            }],
        );

        assert_eq!(report.status, "ready");
        assert!(report.next_actions.is_empty());
    }

    #[test]
    fn readiness_is_ready_to_try_when_game_has_not_started() {
        let report = evaluate_game_readiness(&plan(), &healthy_network(), &[]);

        assert_eq!(report.status, "ready-to-try");
        assert!(report
            .checks
            .iter()
            .any(|check| check.key == "game-port-binding" && check.status == "pending"));
    }

    #[test]
    fn readiness_is_ready_to_try_when_relay_path_is_available_after_p2p_failure() {
        let local = offer("peer_a", vec![candidate("host", "10.0.0.2:39090", 100)]);
        let remote = offer(
            "peer_b",
            vec![
                candidate("srflx", "198.51.100.20:44000", 90),
                candidate("relay", "203.0.113.10:39090", 10),
            ],
        );
        let connection_path = evaluate_connection_path(&local, &remote, "failed");
        let report = evaluate_game_readiness_with_firewall_and_connection_path(
            &plan(),
            &network_with_connected_peers(0),
            &[WindowsNetstatEndpoint {
                protocol: "udp".to_owned(),
                local_address: None,
                local_port: Some(27015),
                foreign_address: None,
                foreign_port: None,
                state: None,
                pid: Some(4242),
            }],
            None,
            Some(&connection_path),
        );

        assert_eq!(report.status, "ready-to-try");
        assert!(report
            .checks
            .iter()
            .any(|check| check.key == "p2p" && check.status == "pending"));
        assert!(report
            .checks
            .iter()
            .any(|check| check.key == "connection-path" && check.status == "pending"));
    }

    #[test]
    fn readiness_marks_firewall_diagnostics_as_blocker() {
        let firewall = FirewallDiagnosticsReport {
            status: "needs-attention".to_owned(),
            summary: "Detected firewall problems.".to_owned(),
            expected_rule_count: 2,
            observed_rule_count: 0,
            problem_count: 2,
            checks: Vec::new(),
        };
        let report = evaluate_game_readiness_with_firewall(
            &plan(),
            &healthy_network(),
            &[WindowsNetstatEndpoint {
                protocol: "udp".to_owned(),
                local_address: None,
                local_port: Some(27015),
                foreign_address: None,
                foreign_port: None,
                state: None,
                pid: Some(4242),
            }],
            Some(&firewall),
        );

        assert_eq!(report.status, "needs-attention");
        assert!(report
            .checks
            .iter()
            .any(|check| check.key == "firewall" && check.status == "needs-attention"));
    }

    #[test]
    fn readiness_marks_runtime_peer_health_as_blocker() {
        let mut network = healthy_network();
        network
            .checks
            .push(crate::network_observation::NetworkObservationCheck {
                key: "runtime-peer:peer_b".to_owned(),
                status: "needs-attention".to_owned(),
                message: "Runtime peer peer_b is missing runtime packets.".to_owned(),
                next_action: "Check that the peer runtime is still running.".to_owned(),
            });
        let report = evaluate_game_readiness(
            &plan(),
            &network,
            &[WindowsNetstatEndpoint {
                protocol: "udp".to_owned(),
                local_address: None,
                local_port: Some(27015),
                foreign_address: None,
                foreign_port: None,
                state: None,
                pid: Some(4242),
            }],
        );

        assert_eq!(report.status, "needs-attention");
        assert!(report.checks.iter().any(|check| {
            check.key == "runtime-peer:peer_b" && check.status == "needs-attention"
        }));
    }

    #[test]
    fn readiness_consumes_runtime_peer_observations_from_network_report() {
        let network = evaluate_network_observations(NetworkObservationSnapshot {
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
            packets: vec![
                PacketObservation {
                    protocol: "udp".to_owned(),
                    source_ip: "10.77.12.2".parse().unwrap(),
                    destination_ip: "10.77.12.255".parse().unwrap(),
                    destination_port: 27015,
                    bytes: 8,
                    direction: "outbound".to_owned(),
                    broadcast: true,
                },
                PacketObservation {
                    protocol: "udp".to_owned(),
                    source_ip: "10.77.12.2".parse().unwrap(),
                    destination_ip: "10.77.12.3".parse().unwrap(),
                    destination_port: 27015,
                    bytes: 8,
                    direction: "outbound".to_owned(),
                    broadcast: false,
                },
            ],
            expected_peer_count: 1,
            expected_broadcast_ports: vec![27015],
            expected_game_ports: vec![27015],
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
        let report = evaluate_game_readiness(
            &plan(),
            &network,
            &[WindowsNetstatEndpoint {
                protocol: "udp".to_owned(),
                local_address: None,
                local_port: Some(27015),
                foreign_address: None,
                foreign_port: None,
                state: None,
                pid: Some(4242),
            }],
        );

        assert_eq!(report.status, "needs-attention");
        assert!(report.checks.iter().any(|check| {
            check.key == "runtime-peer:peer_b" && check.status == "needs-attention"
        }));
    }
}
