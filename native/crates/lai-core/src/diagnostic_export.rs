use crate::firewall_diagnostics::{evaluate_firewall_diagnostics, FirewallDiagnosticsReport};
use crate::game_network_plan::create_game_network_plan;
use crate::game_profile::{CompatibilityLevel, DiscoveryMode, GameProfile};
use crate::ip::Ipv4Subnet;
use crate::network_observation::{
    evaluate_network_observations, AdapterObservation, NetworkObservationReport,
    NetworkObservationSnapshot, PacketObservation, TunnelObservation,
};
use crate::windows_adapter_parser::parse_netsh_adapter_observation;
use crate::windows_firewall_parser::parse_netsh_firewall_rules;
use crate::windows_ping_parser::parse_windows_ping_observation;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticExportEnvironment {
    pub machine_name: String,
    pub user_name: String,
    pub os_version: String,
    pub current_directory: String,
    pub architecture: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticExportInputs {
    pub adapter_name: String,
    pub expected_ip: Option<Ipv4Addr>,
    pub assigned_ip: Option<Ipv4Addr>,
    pub subnet: Option<Ipv4Subnet>,
    pub expected_peers: u16,
    pub ping_host: Option<String>,
    pub packet_observations: Option<String>,
    pub broadcast_ports: Vec<u16>,
    pub game_ports: Vec<u16>,
    pub game_name: String,
    pub discovery: DiscoveryMode,
    pub ports: Vec<u16>,
    pub compatibility: CompatibilityLevel,
    pub program: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DiagnosticTextSource {
    pub source: String,
    pub raw_output: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DiagnosticExportSources {
    pub adapter_netsh: DiagnosticTextSource,
    pub firewall_netsh: DiagnosticTextSource,
    pub ping_output: Option<DiagnosticTextSource>,
    pub packets: Vec<PacketObservation>,
    pub packet_raw_lines: Vec<String>,
    pub packet_error: Option<String>,
    pub packet_io_plan: Option<serde_json::Value>,
    pub packet_io_probe: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticExportBundle {
    pub schema_version: u16,
    pub status: String,
    pub created_at_epoch_ms: u128,
    pub tool: String,
    pub environment: DiagnosticExportEnvironment,
    pub inputs: DiagnosticExportInputs,
    pub adapter_scan: DiagnosticAdapterScanSection,
    pub firewall_scan: DiagnosticFirewallScanSection,
    pub ping: DiagnosticPingSection,
    pub packet_observations: DiagnosticPacketSection,
    pub packet_io: DiagnosticPacketIoSection,
    pub network_observation: NetworkObservationReport,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticAdapterScanSection {
    pub status: String,
    pub source: String,
    pub adapter_name: String,
    pub error: Option<String>,
    pub observation: Option<AdapterObservation>,
    pub raw_output: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticFirewallScanSection {
    pub status: String,
    pub source: String,
    pub error: Option<String>,
    pub observed_rule_count: usize,
    pub diagnosis: FirewallDiagnosticsReport,
    pub raw_output: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticPingSection {
    pub status: String,
    pub source: String,
    pub host: Option<String>,
    pub error: Option<String>,
    pub observation: Option<TunnelObservation>,
    pub raw_output: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticPacketSection {
    pub status: String,
    pub source_file: Option<String>,
    pub broadcast_count: usize,
    pub game_traffic_count: usize,
    pub raw_lines: Vec<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticPacketIoSection {
    pub status: String,
    pub backend: Option<String>,
    pub adapter_read_status: Option<String>,
    pub adapter_write_status: Option<String>,
    pub plan: Option<serde_json::Value>,
    pub probe: Option<serde_json::Value>,
}

pub fn create_diagnostic_export_bundle(
    created_at_epoch_ms: u128,
    environment: DiagnosticExportEnvironment,
    inputs: DiagnosticExportInputs,
    sources: DiagnosticExportSources,
) -> DiagnosticExportBundle {
    let adapter_scan = create_adapter_section(&inputs, &sources.adapter_netsh);
    let firewall_scan = create_firewall_section(&inputs, &sources.firewall_netsh);
    let ping = create_ping_section(&inputs, sources.ping_output.as_ref());
    let packet_observations = create_packet_section(&inputs, &sources);
    let packet_io = create_packet_io_section(&sources);
    let network_observation = evaluate_network_observations(NetworkObservationSnapshot {
        adapter: adapter_scan.observation.clone(),
        tunnel: ping.observation.clone(),
        packets: sources.packets.clone(),
        expected_peer_count: inputs.expected_peers,
        expected_broadcast_ports: inputs.broadcast_ports.clone(),
        expected_game_ports: inputs.game_ports.clone(),
    });
    let status = if [
        adapter_scan.status.as_str(),
        firewall_scan.status.as_str(),
        ping.status.as_str(),
        packet_observations.status.as_str(),
        packet_io.status.as_str(),
        network_observation.status.as_str(),
    ]
    .iter()
    .any(|status| *status == "needs-attention")
    {
        "needs-attention"
    } else {
        "created"
    }
    .to_owned();

    DiagnosticExportBundle {
        schema_version: 2,
        status,
        created_at_epoch_ms,
        tool: "LocalAreaInterconnection Rust CLI".to_owned(),
        environment,
        inputs,
        adapter_scan,
        firewall_scan,
        ping,
        packet_observations,
        packet_io,
        network_observation,
        notes: vec![
            "This bundle is read-only and does not modify Windows Firewall or adapter settings."
                .to_owned(),
            "Raw adapter and firewall data may contain local machine configuration; review before sharing publicly."
                .to_owned(),
        ],
    }
}

fn create_adapter_section(
    inputs: &DiagnosticExportInputs,
    source: &DiagnosticTextSource,
) -> DiagnosticAdapterScanSection {
    let observation = if source.raw_output.trim().is_empty() {
        inputs
            .assigned_ip
            .or(inputs.expected_ip)
            .map(|assigned_ip| AdapterObservation {
                adapter_name: inputs.adapter_name.clone(),
                enabled: true,
                expected_ip: inputs.expected_ip,
                assigned_ip: Some(assigned_ip),
                virtual_subnet: inputs.subnet,
                mtu: None,
                interface_metric: None,
            })
    } else {
        parse_netsh_adapter_observation(
            inputs.adapter_name.clone(),
            &source.raw_output,
            inputs.expected_ip,
            inputs.subnet,
        )
    };
    let status = if source.error.is_none() && observation.is_some() {
        "ok"
    } else {
        "needs-attention"
    }
    .to_owned();

    DiagnosticAdapterScanSection {
        status,
        source: source.source.clone(),
        adapter_name: inputs.adapter_name.clone(),
        error: source.error.clone(),
        observation,
        raw_output: source.raw_output.clone(),
    }
}

fn create_firewall_section(
    inputs: &DiagnosticExportInputs,
    source: &DiagnosticTextSource,
) -> DiagnosticFirewallScanSection {
    let observed_rules = parse_netsh_firewall_rules(&source.raw_output);
    let expected_rules = inputs
        .subnet
        .map(|subnet| {
            let profile = GameProfile {
                game_name: inputs.game_name.clone(),
                steam_app_id: None,
                discovery: inputs.discovery.clone(),
                ports: inputs.ports.clone(),
                join_method: "lan_list_or_direct_ip".to_owned(),
                compatibility: inputs.compatibility.clone(),
                notes: String::new(),
            };
            create_game_network_plan(&profile, subnet, None, None, 30).firewall_rules
        })
        .unwrap_or_default();
    let diagnosis =
        evaluate_firewall_diagnostics(&expected_rules, &observed_rules, inputs.program.as_deref());
    let status = if source.error.is_some() {
        "needs-attention".to_owned()
    } else {
        diagnosis.status.clone()
    };

    DiagnosticFirewallScanSection {
        status,
        source: source.source.clone(),
        error: source.error.clone(),
        observed_rule_count: observed_rules.len(),
        diagnosis,
        raw_output: source.raw_output.clone(),
    }
}

fn create_ping_section(
    inputs: &DiagnosticExportInputs,
    source: Option<&DiagnosticTextSource>,
) -> DiagnosticPingSection {
    let Some(source) = source else {
        return DiagnosticPingSection {
            status: "skipped".to_owned(),
            source: "none".to_owned(),
            host: inputs.ping_host.clone(),
            error: Some("No ping test or ping output was provided.".to_owned()),
            observation: None,
            raw_output: String::new(),
        };
    };
    let observation = if source.error.is_none() && !source.raw_output.trim().is_empty() {
        Some(parse_windows_ping_observation(
            &source.raw_output,
            inputs.expected_peers,
        ))
    } else {
        None
    };
    let status = if source.error.is_none()
        && observation
            .as_ref()
            .is_some_and(|observation| observation.state.eq_ignore_ascii_case("connected"))
    {
        "ok"
    } else {
        "needs-attention"
    }
    .to_owned();

    DiagnosticPingSection {
        status,
        source: source.source.clone(),
        host: inputs.ping_host.clone(),
        error: source.error.clone(),
        observation,
        raw_output: source.raw_output.clone(),
    }
}

fn create_packet_section(
    inputs: &DiagnosticExportInputs,
    sources: &DiagnosticExportSources,
) -> DiagnosticPacketSection {
    DiagnosticPacketSection {
        status: if sources.packet_error.is_none() {
            "ok".to_owned()
        } else {
            "needs-attention".to_owned()
        },
        source_file: inputs.packet_observations.clone(),
        broadcast_count: packet_count(&sources.packets, true, &inputs.broadcast_ports),
        game_traffic_count: packet_count(&sources.packets, false, &inputs.game_ports),
        raw_lines: sources.packet_raw_lines.clone(),
        error: sources.packet_error.clone(),
    }
}

fn packet_count(packets: &[PacketObservation], broadcast: bool, expected_ports: &[u16]) -> usize {
    packets
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

fn create_packet_io_section(sources: &DiagnosticExportSources) -> DiagnosticPacketIoSection {
    let backend = sources
        .packet_io_probe
        .as_ref()
        .and_then(|probe| probe.get("backend"))
        .or_else(|| {
            sources
                .packet_io_plan
                .as_ref()
                .and_then(|plan| plan.get("backend"))
        })
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let adapter_read_status = sources
        .packet_io_probe
        .as_ref()
        .and_then(|probe| probe.get("adapterReadStatus"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let adapter_write_status = sources
        .packet_io_probe
        .as_ref()
        .and_then(|probe| probe.get("adapterWriteStatus"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let probe_status = sources
        .packet_io_probe
        .as_ref()
        .and_then(|probe| probe.get("status"))
        .and_then(serde_json::Value::as_str);
    let status = match probe_status {
        Some("ready") => "ok",
        Some("partial") | Some("unavailable") | Some("unknown-backend") => "needs-attention",
        Some(_) => "ok",
        None if sources.packet_io_plan.is_some() => "skipped",
        None => "skipped",
    }
    .to_owned();

    DiagnosticPacketIoSection {
        status,
        backend,
        adapter_read_status,
        adapter_write_status,
        plan: sources.packet_io_plan.clone(),
        probe: sources.packet_io_probe.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> DiagnosticExportEnvironment {
        DiagnosticExportEnvironment {
            machine_name: "test-machine".to_owned(),
            user_name: "tester".to_owned(),
            os_version: "windows-test".to_owned(),
            current_directory: "D:\\work".to_owned(),
            architecture: "x86_64".to_owned(),
        }
    }

    fn inputs() -> DiagnosticExportInputs {
        DiagnosticExportInputs {
            adapter_name: "LocalAreaInterconnection".to_owned(),
            expected_ip: Some("10.77.12.2".parse().unwrap()),
            assigned_ip: Some("10.77.12.2".parse().unwrap()),
            subnet: Some("10.77.12.0/24".parse().unwrap()),
            expected_peers: 1,
            ping_host: Some("10.77.12.1".to_owned()),
            packet_observations: Some("packets.txt".to_owned()),
            broadcast_ports: vec![39078],
            game_ports: vec![39077],
            game_name: "Example Game".to_owned(),
            discovery: DiscoveryMode::UdpBroadcast,
            ports: vec![39077],
            compatibility: CompatibilityLevel::Unknown,
            program: None,
        }
    }

    fn packet(port: u16, broadcast: bool) -> PacketObservation {
        PacketObservation {
            protocol: "udp".to_owned(),
            source_ip: "10.77.12.2".parse().unwrap(),
            destination_ip: if broadcast {
                "10.77.12.255".parse().unwrap()
            } else {
                "10.77.12.1".parse().unwrap()
            },
            destination_port: port,
            bytes: 8,
            direction: "outbound".to_owned(),
            broadcast,
        }
    }

    #[test]
    fn diagnostic_bundle_combines_sections_and_network_observation() {
        let adapter = DiagnosticTextSource {
            source: "netsh-file".to_owned(),
            raw_output: r#"
Configuration for interface "LocalAreaInterconnection"
    IP Address:                           10.77.12.2
    Subnet Prefix:                        10.77.12.0/24 (mask 255.255.255.0)
"#
            .to_owned(),
            error: None,
        };
        let firewall = DiagnosticTextSource {
            source: "netsh-file".to_owned(),
            raw_output: r#"
Rule Name:                            Example UDP 39077
Enabled:                              Yes
Direction:                            In
Profiles:                             Private
RemoteIP:                             10.77.12.0/24
Protocol:                             UDP
LocalPort:                            39077
Action:                               Allow
"#
            .to_owned(),
            error: None,
        };
        let ping = DiagnosticTextSource {
            source: "ping-file".to_owned(),
            raw_output: r#"
Ping statistics for 10.77.12.1:
    Packets: Sent = 4, Received = 4, Lost = 0 (0% loss),
Approximate round trip times in milli-seconds:
    Minimum = 0ms, Maximum = 1ms, Average = 0ms
"#
            .to_owned(),
            error: None,
        };
        let bundle = create_diagnostic_export_bundle(
            123,
            env(),
            inputs(),
            DiagnosticExportSources {
                adapter_netsh: adapter,
                firewall_netsh: firewall,
                ping_output: Some(ping),
                packets: vec![packet(39078, true), packet(39077, false)],
                packet_raw_lines: vec![
                    "udp:10.77.12.2:10.77.12.255:39078:broadcast:outbound:8".to_owned(),
                    "udp:10.77.12.2:10.77.12.1:39077:unicast:outbound:8".to_owned(),
                ],
                packet_error: None,
                packet_io_plan: Some(serde_json::json!({
                    "backend": "userspace-udp",
                    "can_read_ipv4": false
                })),
                packet_io_probe: Some(serde_json::json!({
                    "backend": "userspace-udp",
                    "status": "ready",
                    "adapterReadStatus": "not-required",
                    "adapterWriteStatus": "not-required"
                })),
            },
        );

        assert_eq!(bundle.schema_version, 2);
        assert_eq!(bundle.status, "needs-attention");
        assert_eq!(bundle.adapter_scan.status, "ok");
        assert_eq!(bundle.ping.status, "ok");
        assert_eq!(bundle.packet_io.status, "ok");
        assert_eq!(bundle.packet_io.backend.as_deref(), Some("userspace-udp"));
        assert_eq!(bundle.packet_observations.broadcast_count, 1);
        assert_eq!(bundle.packet_observations.game_traffic_count, 1);
        assert_eq!(
            bundle
                .network_observation
                .diagnostic_snapshot
                .broadcast
                .as_deref(),
            Some("seen")
        );
    }
}
