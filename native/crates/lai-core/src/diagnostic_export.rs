use crate::broadcast_policy::BroadcastForwardReport;
use crate::connection_path::ConnectionPathReport;
use crate::firewall_diagnostics::{evaluate_firewall_diagnostics, FirewallDiagnosticsReport};
use crate::game_network_plan::{create_game_network_plan, GameNetworkPlan};
use crate::game_profile::{CompatibilityLevel, DiscoveryMode, GameProfile};
use crate::game_readiness::{
    evaluate_game_readiness_with_firewall_and_connection_path, GameReadinessReport,
};
use crate::ip::Ipv4Subnet;
use crate::network_observation::{
    evaluate_network_observations, AdapterObservation, NetworkObservationReport,
    NetworkObservationSnapshot, PacketObservation, RuntimePeerObservation, TunnelObservation,
};
use crate::relay_fallback_plan::RelayFallbackPlan;
use crate::runtime_cleanup_plan::{
    create_runtime_cleanup_report, RuntimeCleanupPlan, RuntimeCleanupReport,
};
use crate::windows_adapter_parser::parse_netsh_adapter_observation;
use crate::windows_firewall_parser::parse_netsh_firewall_rules;
use crate::windows_netstat_parser::{parse_windows_netstat_ano, WindowsNetstatEndpoint};
use crate::windows_ping_parser::parse_windows_ping_observation;
use crate::windows_route_parser::{parse_windows_ipv4_routes, WindowsRouteObservation};
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
    pub runtime_snapshot: Option<serde_json::Value>,
    pub runtime_snapshot_error: Option<String>,
    pub route_table: DiagnosticTextSource,
    pub netstat_table: DiagnosticTextSource,
    pub relay_fallback_plan: Option<RelayFallbackPlan>,
    pub connection_path_report: Option<ConnectionPathReport>,
    pub relay_fallback_error: Option<String>,
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
    pub broadcast_forward: DiagnosticBroadcastForwardSection,
    pub route_scan: DiagnosticRouteScanSection,
    pub game_port_scan: DiagnosticGamePortScanSection,
    pub game_readiness: GameReadinessReport,
    pub runtime_cleanup: DiagnosticRuntimeCleanupSection,
    pub runtime_peers: DiagnosticRuntimePeersSection,
    pub relay_fallback: DiagnosticRelayFallbackSection,
    pub connection_path: DiagnosticConnectionPathSection,
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
    pub runtime: DiagnosticRuntimePacketIoEvidence,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticBroadcastForwardSection {
    pub status: String,
    pub source: String,
    pub event_count: usize,
    pub forwarded_event_count: usize,
    pub dropped_event_count: usize,
    pub forwarded_target_count: usize,
    pub rate_limited_count: usize,
    pub report: Option<BroadcastForwardReport>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticRouteScanSection {
    pub status: String,
    pub source: String,
    pub error: Option<String>,
    pub route_count: usize,
    pub room_route_count: usize,
    pub routes: Vec<WindowsRouteObservation>,
    pub room_routes: Vec<WindowsRouteObservation>,
    pub raw_output: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticGamePortScanSection {
    pub status: String,
    pub source: String,
    pub error: Option<String>,
    pub endpoint_count: usize,
    pub match_count: usize,
    pub expected_ports: Vec<u16>,
    pub matches: Vec<WindowsNetstatEndpoint>,
    pub raw_output: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DiagnosticRuntimePacketIoEvidence {
    pub status: Option<String>,
    pub error: Option<String>,
    pub raw_virtual_packet_count: usize,
    pub forwarded_packet_count: usize,
    pub injected_packet_count: usize,
    pub wintun_received_packet_count: usize,
    pub wintun_sent_packet_count: usize,
    pub wintun_error_count: usize,
    pub raw_virtual_packets: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticRuntimeCleanupSection {
    pub status: String,
    pub error: Option<String>,
    pub requires_elevation: bool,
    pub restore_adapter: bool,
    pub process_step_count: usize,
    pub command_count: usize,
    pub check_count: usize,
    pub next_action_count: usize,
    pub route_count: usize,
    pub route_error: Option<String>,
    pub plan: Option<RuntimeCleanupPlan>,
    pub report: Option<RuntimeCleanupReport>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticRuntimePeersSection {
    pub status: String,
    pub source: String,
    pub error: Option<String>,
    pub peer_count: usize,
    pub connected_peer_count: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_direct_bytes_sent: u64,
    pub total_direct_bytes_received: u64,
    pub total_relay_bytes_sent: u64,
    pub total_relay_bytes_received: u64,
    pub summaries: Vec<DiagnosticRuntimePeerSummary>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticRuntimePeerSummary {
    pub peer_id: String,
    pub virtual_ip: String,
    pub endpoint: String,
    pub selected_path: String,
    pub connection_path_status: String,
    pub bootstrap_status: String,
    pub connected: bool,
    #[serde(default)]
    pub path_kind: Option<String>,
    pub latency_ms: Option<u64>,
    pub last_seen_at_ms: Option<u64>,
    pub last_sent_at_ms: Option<u64>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    #[serde(default)]
    pub direct_bytes_sent: u64,
    #[serde(default)]
    pub direct_bytes_received: u64,
    #[serde(default)]
    pub relay_bytes_sent: u64,
    #[serde(default)]
    pub relay_bytes_received: u64,
    #[serde(default)]
    pub unknown_path_bytes_sent: u64,
    #[serde(default)]
    pub unknown_path_bytes_received: u64,
    pub heartbeat_packets_sent: u64,
    pub heartbeat_ack_packets_received: u64,
    pub heartbeat_ack_packets_sent: u64,
    pub heartbeat_loss_percent: Option<f64>,
    #[serde(default)]
    pub heartbeat_loss_window_size: usize,
    #[serde(default)]
    pub heartbeat_loss_window_percent: Option<f64>,
    #[serde(default)]
    pub heartbeat_rtt_sample_count: usize,
    #[serde(default)]
    pub heartbeat_rtt_jitter_ms: Option<f64>,
    pub forwarded_packets_sent: u64,
    pub tunnel_packets_received: u64,
    #[serde(default)]
    pub health: Option<DiagnosticRuntimePeerHealth>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticRuntimePeerHealth {
    pub status: String,
    pub reason: String,
    pub next_action: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticRelayFallbackSection {
    pub status: String,
    pub error: Option<String>,
    pub plan: Option<RelayFallbackPlan>,
    #[serde(default)]
    pub runtime_summary_count: usize,
    #[serde(default)]
    pub runtime_summaries: Vec<DiagnosticRuntimeRelayFallbackSummary>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DiagnosticRuntimeRelayFallbackSummary {
    pub source: String,
    pub peer_id: String,
    pub bootstrap_status: String,
    pub status: String,
    pub selected_path: String,
    pub p2p_status: String,
    pub p2p_candidate_count: u64,
    pub relay_candidate_count: u64,
    pub selected_relay_endpoints: Vec<String>,
    pub recommended_actions: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiagnosticConnectionPathSection {
    pub status: String,
    pub source: String,
    pub error: Option<String>,
    pub runtime_path: Option<String>,
    pub report: Option<ConnectionPathReport>,
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
    let broadcast_forward = create_broadcast_forward_section(&sources);
    let route_scan = create_route_section(&inputs, &sources.route_table);
    let game_port_scan = create_game_port_section(&inputs, &sources.netstat_table);
    let runtime_cleanup =
        create_runtime_cleanup_section(&sources, adapter_scan.observation.clone());
    let game_plan = create_export_game_plan(&inputs);
    let relay_fallback = create_relay_fallback_section(&sources);
    let connection_path = create_connection_path_section(&sources);
    let runtime_peers = create_runtime_peers_section(&sources);
    let tunnel_observation = tunnel_observation_with_connection_path(
        ping.observation.clone(),
        connection_path.report.as_ref(),
        connection_path.runtime_path.as_deref(),
    );
    let network_observation = evaluate_network_observations(NetworkObservationSnapshot {
        adapter: adapter_scan.observation.clone(),
        tunnel: tunnel_observation,
        packets: sources.packets.clone(),
        expected_peer_count: inputs.expected_peers,
        expected_broadcast_ports: inputs.broadcast_ports.clone(),
        expected_game_ports: inputs.game_ports.clone(),
        route_observations: route_scan.routes.clone(),
        runtime_peers: runtime_peer_observations(&runtime_peers),
    });
    let game_readiness = evaluate_game_readiness_with_firewall_and_connection_path(
        &game_plan,
        &network_observation,
        &game_port_scan.matches,
        Some(&firewall_scan.diagnosis),
        connection_path.report.as_ref(),
    );
    let status = if [
        adapter_scan.status.as_str(),
        firewall_scan.status.as_str(),
        ping.status.as_str(),
        packet_observations.status.as_str(),
        packet_io.status.as_str(),
        broadcast_forward.status.as_str(),
        route_scan.status.as_str(),
        game_port_scan.status.as_str(),
        game_readiness.status.as_str(),
        runtime_cleanup.status.as_str(),
        runtime_peers.status.as_str(),
        relay_fallback.status.as_str(),
        connection_path.status.as_str(),
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
        schema_version: 18,
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
        broadcast_forward,
        route_scan,
        game_port_scan,
        game_readiness,
        runtime_cleanup,
        runtime_peers,
        relay_fallback,
        connection_path,
        network_observation,
        notes: vec![
            "This bundle is read-only and does not modify Windows Firewall or adapter settings."
                .to_owned(),
            "Raw adapter and firewall data may contain local machine configuration; review before sharing publicly."
                .to_owned(),
        ],
    }
}

fn tunnel_observation_with_connection_path(
    tunnel: Option<TunnelObservation>,
    connection_path: Option<&ConnectionPathReport>,
    runtime_path: Option<&str>,
) -> Option<TunnelObservation> {
    tunnel.map(|mut observation| {
        if let Some(report) = connection_path {
            observation.path = Some(report.selected_path.clone());
        } else if observation
            .path
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            observation.path = runtime_path.map(str::to_owned);
        }
        observation
    })
}

fn create_relay_fallback_section(
    sources: &DiagnosticExportSources,
) -> DiagnosticRelayFallbackSection {
    let runtime_summaries = runtime_relay_fallback_summaries(sources.runtime_snapshot.as_ref());
    let runtime_summary_count = runtime_summaries.len();
    if let Some(error) = sources.relay_fallback_error.clone() {
        return DiagnosticRelayFallbackSection {
            status: "needs-attention".to_owned(),
            error: Some(error),
            plan: sources.relay_fallback_plan.clone(),
            runtime_summary_count,
            runtime_summaries,
        };
    }
    let plan = sources.relay_fallback_plan.clone().or_else(|| {
        runtime_connection_path_report(sources.runtime_snapshot.as_ref())
            .map(|report| report.relay_fallback)
    });
    let Some(plan) = plan else {
        return DiagnosticRelayFallbackSection {
            status: "skipped".to_owned(),
            error: Some("No local and remote NAT offers were provided.".to_owned()),
            plan: None,
            runtime_summary_count,
            runtime_summaries,
        };
    };
    let status = match plan.status.as_str() {
        "p2p-ready" | "relay-available" => "ok",
        _ => "needs-attention",
    }
    .to_owned();
    DiagnosticRelayFallbackSection {
        status,
        error: None,
        plan: Some(plan),
        runtime_summary_count,
        runtime_summaries,
    }
}

fn create_connection_path_section(
    sources: &DiagnosticExportSources,
) -> DiagnosticConnectionPathSection {
    let runtime_path = runtime_connection_path(sources.runtime_snapshot.as_ref());
    let runtime_report = runtime_connection_path_report(sources.runtime_snapshot.as_ref());
    if let Some(error) = sources.relay_fallback_error.clone() {
        return DiagnosticConnectionPathSection {
            status: "needs-attention".to_owned(),
            source: "error".to_owned(),
            error: Some(error),
            runtime_path,
            report: sources.connection_path_report.clone(),
        };
    }
    let report_source = if sources.connection_path_report.is_some() {
        "nat-offers"
    } else {
        "runtime-snapshot-report"
    };
    let Some(report) = sources.connection_path_report.clone().or(runtime_report) else {
        if let Some(path) = runtime_path.clone() {
            let status = if matches!(path.as_str(), "p2p" | "relay") {
                "ok"
            } else {
                "needs-attention"
            }
            .to_owned();
            return DiagnosticConnectionPathSection {
                status,
                source: "runtime-snapshot".to_owned(),
                error: None,
                runtime_path: Some(path),
                report: None,
            };
        }
        return DiagnosticConnectionPathSection {
            status: "skipped".to_owned(),
            source: "skipped".to_owned(),
            error: Some("No local and remote NAT offers were provided.".to_owned()),
            runtime_path: None,
            report: None,
        };
    };
    let status = match report.status.as_str() {
        "p2p-candidate-ready" | "relay-ready" => "ok",
        _ => "needs-attention",
    }
    .to_owned();
    DiagnosticConnectionPathSection {
        status,
        source: report_source.to_owned(),
        error: None,
        runtime_path,
        report: Some(report),
    }
}

fn runtime_connection_path(runtime_snapshot: Option<&serde_json::Value>) -> Option<String> {
    runtime_snapshot
        .and_then(|snapshot| snapshot.get("tunnelServiceSnapshot"))
        .and_then(|snapshot| snapshot.get("connection_path"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(|path| path.to_ascii_lowercase())
}

fn runtime_connection_path_report(
    runtime_snapshot: Option<&serde_json::Value>,
) -> Option<ConnectionPathReport> {
    let reports = runtime_snapshot?
        .get("connectionPathReports")?
        .as_array()?
        .iter()
        .filter_map(|entry| {
            let value = entry
                .get("report")
                .cloned()
                .unwrap_or_else(|| entry.clone());
            serde_json::from_value::<ConnectionPathReport>(value).ok()
        })
        .collect::<Vec<_>>();
    reports
        .iter()
        .find(|report| {
            matches!(
                report.status.as_str(),
                "p2p-candidate-ready" | "relay-ready"
            )
        })
        .cloned()
        .or_else(|| reports.into_iter().next())
}

fn runtime_relay_fallback_summaries(
    runtime_snapshot: Option<&serde_json::Value>,
) -> Vec<DiagnosticRuntimeRelayFallbackSummary> {
    let Some(snapshot) = runtime_snapshot else {
        return Vec::new();
    };
    let explicit = json_array(snapshot, "runtimeRelayFallbackSummaries")
        .into_iter()
        .filter_map(|summary| serde_json::from_value(summary).ok())
        .collect::<Vec<_>>();
    if !explicit.is_empty() {
        return explicit;
    }
    json_array(snapshot, "connectionPathReports")
        .into_iter()
        .filter_map(runtime_relay_fallback_summary_from_connection_path_entry)
        .collect()
}

fn runtime_relay_fallback_summary_from_connection_path_entry(
    entry: serde_json::Value,
) -> Option<DiagnosticRuntimeRelayFallbackSummary> {
    let report = entry
        .get("report")
        .cloned()
        .unwrap_or_else(|| entry.clone());
    let fallback = report.get("relay_fallback")?;
    Some(DiagnosticRuntimeRelayFallbackSummary {
        source: entry
            .get("source")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("connection-path")
            .to_owned(),
        peer_id: entry
            .get("peerId")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                report
                    .get("remote_peer_id")
                    .and_then(serde_json::Value::as_str)
            })
            .unwrap_or("unknown")
            .to_owned(),
        bootstrap_status: entry
            .get("bootstrapStatus")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned(),
        status: fallback
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned(),
        selected_path: report
            .get("selected_path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned(),
        p2p_status: fallback
            .get("p2p_status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned(),
        p2p_candidate_count: fallback
            .get("p2p_candidate_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default(),
        relay_candidate_count: fallback
            .get("relay_candidate_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default(),
        selected_relay_endpoints: json_string_array(fallback, "selected_relay_endpoints"),
        recommended_actions: json_string_array(fallback, "recommended_actions"),
        warnings: json_string_array(fallback, "warnings"),
    })
}

fn create_export_game_plan(inputs: &DiagnosticExportInputs) -> GameNetworkPlan {
    let profile = GameProfile {
        game_name: inputs.game_name.clone(),
        steam_app_id: None,
        discovery: inputs.discovery.clone(),
        ports: inputs.ports.clone(),
        join_method: "lan_list_or_direct_ip".to_owned(),
        compatibility: inputs.compatibility.clone(),
        notes: String::new(),
    };
    let subnet = inputs.subnet.unwrap_or(Ipv4Subnet {
        network: Ipv4Addr::new(10, 77, 0, 0),
        prefix: 16,
    });
    create_game_network_plan(
        &profile,
        subnet,
        None,
        inputs.assigned_ip.or(inputs.expected_ip),
        30,
    )
}

fn create_runtime_cleanup_section(
    sources: &DiagnosticExportSources,
    adapter_observation: Option<AdapterObservation>,
) -> DiagnosticRuntimeCleanupSection {
    if let Some(error) = sources.runtime_snapshot_error.clone() {
        return DiagnosticRuntimeCleanupSection {
            status: "needs-attention".to_owned(),
            error: Some(error),
            requires_elevation: false,
            restore_adapter: false,
            process_step_count: 0,
            command_count: 0,
            check_count: 0,
            next_action_count: 0,
            route_count: 0,
            route_error: sources.route_table.error.clone(),
            plan: None,
            report: None,
        };
    }

    let Some(plan_value) = sources
        .runtime_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.get("runtimeCleanupPlan"))
        .cloned()
    else {
        return DiagnosticRuntimeCleanupSection {
            status: "skipped".to_owned(),
            error: Some("No runtime cleanup plan was present in the runtime snapshot.".to_owned()),
            requires_elevation: false,
            restore_adapter: false,
            process_step_count: 0,
            command_count: 0,
            check_count: 0,
            next_action_count: 0,
            route_count: 0,
            route_error: sources.route_table.error.clone(),
            plan: None,
            report: None,
        };
    };

    match serde_json::from_value::<RuntimeCleanupPlan>(plan_value) {
        Ok(plan) => {
            let routes = if sources.route_table.error.is_none() {
                parse_windows_ipv4_routes(&sources.route_table.raw_output)
            } else {
                Vec::new()
            };
            let route_count = routes.len();
            let report = create_runtime_cleanup_report(
                plan.clone(),
                adapter_observation,
                routes,
                runtime_wintun_close_report(sources.runtime_snapshot.as_ref()),
            );
            let status = if sources.route_table.error.is_some() {
                "needs-attention".to_owned()
            } else {
                report.status.clone()
            };
            DiagnosticRuntimeCleanupSection {
                status,
                error: sources.route_table.error.clone(),
                requires_elevation: plan.requires_elevation,
                restore_adapter: plan.restore_adapter,
                process_step_count: plan.process_cleanup_steps.len(),
                command_count: plan.commands.len(),
                check_count: report.checks.len(),
                next_action_count: report.next_actions.len(),
                route_count,
                route_error: sources.route_table.error.clone(),
                plan: Some(plan),
                report: Some(report),
            }
        }
        Err(err) => DiagnosticRuntimeCleanupSection {
            status: "needs-attention".to_owned(),
            error: Some(format!("Runtime cleanup plan could not be parsed: {err}")),
            requires_elevation: false,
            restore_adapter: false,
            process_step_count: 0,
            command_count: 0,
            check_count: 0,
            next_action_count: 0,
            route_count: 0,
            route_error: sources.route_table.error.clone(),
            plan: None,
            report: None,
        },
    }
}

fn create_runtime_peers_section(
    sources: &DiagnosticExportSources,
) -> DiagnosticRuntimePeersSection {
    if let Some(error) = sources.runtime_snapshot_error.clone() {
        return DiagnosticRuntimePeersSection {
            status: "needs-attention".to_owned(),
            source: "runtime-snapshot".to_owned(),
            error: Some(error),
            peer_count: 0,
            connected_peer_count: 0,
            total_bytes_sent: 0,
            total_bytes_received: 0,
            total_direct_bytes_sent: 0,
            total_direct_bytes_received: 0,
            total_relay_bytes_sent: 0,
            total_relay_bytes_received: 0,
            summaries: Vec::new(),
        };
    }
    let Some(snapshot) = sources.runtime_snapshot.as_ref() else {
        return DiagnosticRuntimePeersSection {
            status: "skipped".to_owned(),
            source: "skipped".to_owned(),
            error: Some("No runtime snapshot was provided.".to_owned()),
            peer_count: 0,
            connected_peer_count: 0,
            total_bytes_sent: 0,
            total_bytes_received: 0,
            total_direct_bytes_sent: 0,
            total_direct_bytes_received: 0,
            total_relay_bytes_sent: 0,
            total_relay_bytes_received: 0,
            summaries: Vec::new(),
        };
    };
    let summaries = json_array(snapshot, "runtimePeerSummaries")
        .into_iter()
        .filter_map(|value| serde_json::from_value::<DiagnosticRuntimePeerSummary>(value).ok())
        .map(|mut summary| {
            summary.health = Some(runtime_peer_health(&summary));
            summary
        })
        .collect::<Vec<_>>();
    if summaries.is_empty() {
        return DiagnosticRuntimePeersSection {
            status: "skipped".to_owned(),
            source: "runtime-snapshot".to_owned(),
            error: Some("No runtimePeerSummaries were present in the runtime snapshot.".to_owned()),
            peer_count: 0,
            connected_peer_count: 0,
            total_bytes_sent: 0,
            total_bytes_received: 0,
            total_direct_bytes_sent: 0,
            total_direct_bytes_received: 0,
            total_relay_bytes_sent: 0,
            total_relay_bytes_received: 0,
            summaries,
        };
    }
    let connected_peer_count = summaries.iter().filter(|summary| summary.connected).count();
    let total_bytes_sent = summaries.iter().map(|summary| summary.bytes_sent).sum();
    let total_bytes_received = summaries.iter().map(|summary| summary.bytes_received).sum();
    let total_direct_bytes_sent = summaries
        .iter()
        .map(|summary| summary.direct_bytes_sent)
        .sum();
    let total_direct_bytes_received = summaries
        .iter()
        .map(|summary| summary.direct_bytes_received)
        .sum();
    let total_relay_bytes_sent = summaries
        .iter()
        .map(|summary| summary.relay_bytes_sent)
        .sum();
    let total_relay_bytes_received = summaries
        .iter()
        .map(|summary| summary.relay_bytes_received)
        .sum();
    let needs_attention = summaries.iter().any(|summary| {
        summary
            .health
            .as_ref()
            .is_some_and(|health| health.status != "ok")
    });
    DiagnosticRuntimePeersSection {
        status: if needs_attention {
            "needs-attention"
        } else {
            "ok"
        }
        .to_owned(),
        source: "runtime-snapshot".to_owned(),
        error: None,
        peer_count: summaries.len(),
        connected_peer_count,
        total_bytes_sent,
        total_bytes_received,
        total_direct_bytes_sent,
        total_direct_bytes_received,
        total_relay_bytes_sent,
        total_relay_bytes_received,
        summaries,
    }
}

fn runtime_peer_health(summary: &DiagnosticRuntimePeerSummary) -> DiagnosticRuntimePeerHealth {
    let (status, reason, next_action) = if matches!(
        summary.connection_path_status.as_str(),
        "no-path" | "needs-relay" | "config-error"
    ) || matches!(
        summary.selected_path.as_str(),
        "none" | "failed"
    ) {
        (
            "needs-attention",
            "no-usable-path",
            "Refresh NAT candidates or configure relay before starting the game.",
        )
    } else if !summary.connected && summary.heartbeat_ack_packets_received == 0 {
        (
            "needs-attention",
            "no-runtime-packets",
            "Check that the peer runtime is still running and reachable on its tunnel endpoint.",
        )
    } else if summary
        .heartbeat_loss_percent
        .is_some_and(|loss| loss >= 50.0)
    {
        (
            "needs-attention",
            "heartbeat-loss-high",
            "Check firewall, NAT mapping, or relay fallback; heartbeat acknowledgements are missing.",
        )
    } else if summary.latency_ms.is_some_and(|latency| latency >= 150) {
        (
            "degraded",
            "latency-high",
            "Direct IP may work, but expect delay; consider relay region or network changes.",
        )
    } else {
        (
            "ok",
            "runtime-peer-healthy",
            "Peer runtime path, heartbeat, and traffic evidence look healthy.",
        )
    };
    DiagnosticRuntimePeerHealth {
        status: status.to_owned(),
        reason: reason.to_owned(),
        next_action: next_action.to_owned(),
    }
}

fn runtime_peer_observations(
    runtime_peers: &DiagnosticRuntimePeersSection,
) -> Vec<RuntimePeerObservation> {
    runtime_peers
        .summaries
        .iter()
        .map(|summary| RuntimePeerObservation {
            peer_id: summary.peer_id.clone(),
            virtual_ip: summary.virtual_ip.clone(),
            selected_path: summary.selected_path.clone(),
            connection_path_status: summary.connection_path_status.clone(),
            bootstrap_status: summary.bootstrap_status.clone(),
            connected: summary.connected,
            path_kind: summary.path_kind.clone(),
            latency_ms: summary.latency_ms,
            last_seen_at_ms: summary.last_seen_at_ms,
            last_sent_at_ms: summary.last_sent_at_ms,
            bytes_sent: summary.bytes_sent,
            bytes_received: summary.bytes_received,
            direct_bytes_sent: summary.direct_bytes_sent,
            direct_bytes_received: summary.direct_bytes_received,
            relay_bytes_sent: summary.relay_bytes_sent,
            relay_bytes_received: summary.relay_bytes_received,
            unknown_path_bytes_sent: summary.unknown_path_bytes_sent,
            unknown_path_bytes_received: summary.unknown_path_bytes_received,
            heartbeat_packets_sent: summary.heartbeat_packets_sent,
            heartbeat_ack_packets_received: summary.heartbeat_ack_packets_received,
            heartbeat_loss_percent: summary.heartbeat_loss_percent,
            heartbeat_loss_window_size: summary.heartbeat_loss_window_size,
            heartbeat_loss_window_percent: summary.heartbeat_loss_window_percent,
            heartbeat_rtt_sample_count: summary.heartbeat_rtt_sample_count,
            heartbeat_rtt_jitter_ms: summary.heartbeat_rtt_jitter_ms,
            forwarded_packets_sent: summary.forwarded_packets_sent,
            tunnel_packets_received: summary.tunnel_packets_received,
        })
        .collect()
}

fn create_route_section(
    inputs: &DiagnosticExportInputs,
    source: &DiagnosticTextSource,
) -> DiagnosticRouteScanSection {
    if source.source == "skipped" && source.raw_output.trim().is_empty() {
        return DiagnosticRouteScanSection {
            status: "skipped".to_owned(),
            source: source.source.clone(),
            error: source.error.clone(),
            route_count: 0,
            room_route_count: 0,
            routes: Vec::new(),
            room_routes: Vec::new(),
            raw_output: String::new(),
        };
    }

    let routes = if source.error.is_none() {
        parse_windows_ipv4_routes(&source.raw_output)
    } else {
        Vec::new()
    };
    let room_routes = routes
        .iter()
        .filter(|route| route_matches_inputs(route, inputs))
        .cloned()
        .collect::<Vec<_>>();
    let status = if source.error.is_some() {
        "needs-attention"
    } else {
        "ok"
    }
    .to_owned();

    DiagnosticRouteScanSection {
        status,
        source: source.source.clone(),
        error: source.error.clone(),
        route_count: routes.len(),
        room_route_count: room_routes.len(),
        routes,
        room_routes,
        raw_output: source.raw_output.clone(),
    }
}

fn create_game_port_section(
    inputs: &DiagnosticExportInputs,
    source: &DiagnosticTextSource,
) -> DiagnosticGamePortScanSection {
    if source.source == "skipped" && source.raw_output.trim().is_empty() {
        return DiagnosticGamePortScanSection {
            status: "skipped".to_owned(),
            source: source.source.clone(),
            error: source.error.clone(),
            endpoint_count: 0,
            match_count: 0,
            expected_ports: inputs.ports.clone(),
            matches: Vec::new(),
            raw_output: String::new(),
        };
    }

    let endpoints = if source.error.is_none() {
        parse_windows_netstat_ano(&source.raw_output)
    } else {
        Vec::new()
    };
    let matches = endpoints
        .iter()
        .filter(|endpoint| {
            endpoint
                .local_port
                .is_some_and(|port| inputs.ports.contains(&port))
        })
        .cloned()
        .collect::<Vec<_>>();
    let status = if source.error.is_some() {
        "needs-attention"
    } else if inputs.ports.is_empty() {
        "skipped"
    } else if matches.is_empty() {
        "needs-attention"
    } else {
        "ok"
    }
    .to_owned();

    DiagnosticGamePortScanSection {
        status,
        source: source.source.clone(),
        error: source.error.clone(),
        endpoint_count: endpoints.len(),
        match_count: matches.len(),
        expected_ports: inputs.ports.clone(),
        matches,
        raw_output: source.raw_output.clone(),
    }
}

fn route_matches_inputs(route: &WindowsRouteObservation, inputs: &DiagnosticExportInputs) -> bool {
    if route.destination.prefix == 0 {
        return false;
    }
    inputs
        .expected_ip
        .is_some_and(|ip| route.destination.contains(ip))
        || inputs
            .subnet
            .is_some_and(|subnet| route.destination.intersects(subnet))
}

fn runtime_wintun_close_report(
    runtime_snapshot: Option<&serde_json::Value>,
) -> Option<crate::wintun_runtime::WintunPacketIoCloseReport> {
    runtime_snapshot
        .and_then(|snapshot| snapshot.get("wintunRuntime"))
        .and_then(|runtime| runtime.get("close"))
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
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
    let packet_io_plan = sources.packet_io_plan.clone().or_else(|| {
        sources
            .runtime_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.get("packetIoPlan"))
            .cloned()
    });
    let packet_io_probe = sources.packet_io_probe.clone().or_else(|| {
        sources
            .runtime_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.get("packetIoProbe"))
            .cloned()
    });
    let backend = sources
        .packet_io_probe
        .as_ref()
        .or(packet_io_probe.as_ref())
        .and_then(|probe| probe.get("backend"))
        .or_else(|| {
            sources
                .packet_io_plan
                .as_ref()
                .or(packet_io_plan.as_ref())
                .and_then(|plan| plan.get("backend"))
        })
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let adapter_read_status = sources
        .packet_io_probe
        .as_ref()
        .or(packet_io_probe.as_ref())
        .and_then(|probe| probe.get("adapterReadStatus"))
        .or_else(|| {
            sources
                .runtime_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.get("adapterReadStatus"))
        })
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let adapter_write_status = sources
        .packet_io_probe
        .as_ref()
        .or(packet_io_probe.as_ref())
        .and_then(|probe| probe.get("adapterWriteStatus"))
        .or_else(|| {
            sources
                .runtime_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.get("adapterWriteStatus"))
        })
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let probe_status = sources
        .packet_io_probe
        .as_ref()
        .or(packet_io_probe.as_ref())
        .and_then(|probe| probe.get("status"))
        .and_then(serde_json::Value::as_str);
    let runtime = create_runtime_packet_io_evidence(sources);
    let status = if runtime.error.is_some() {
        "needs-attention".to_owned()
    } else {
        match probe_status {
            Some("ready") => "ok",
            Some("partial") | Some("unavailable") | Some("unknown-backend") => "needs-attention",
            Some(_) => "ok",
            None if packet_io_plan.is_some() => "skipped",
            None => "skipped",
        }
        .to_owned()
    };

    DiagnosticPacketIoSection {
        status,
        backend,
        adapter_read_status,
        adapter_write_status,
        plan: packet_io_plan,
        probe: packet_io_probe,
        runtime,
    }
}

fn create_broadcast_forward_section(
    sources: &DiagnosticExportSources,
) -> DiagnosticBroadcastForwardSection {
    let report = sources
        .runtime_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.get("broadcastForwardReport"))
        .cloned()
        .and_then(|value| serde_json::from_value::<BroadcastForwardReport>(value).ok());
    let Some(report) = report else {
        return DiagnosticBroadcastForwardSection {
            status: if sources.runtime_snapshot_error.is_some() {
                "needs-attention".to_owned()
            } else {
                "skipped".to_owned()
            },
            source: if sources.runtime_snapshot.is_some() {
                "runtime-snapshot".to_owned()
            } else {
                "skipped".to_owned()
            },
            event_count: 0,
            forwarded_event_count: 0,
            dropped_event_count: 0,
            forwarded_target_count: 0,
            rate_limited_count: 0,
            report: None,
        };
    };
    DiagnosticBroadcastForwardSection {
        status: report.status.clone(),
        source: "runtime-snapshot".to_owned(),
        event_count: report.event_count,
        forwarded_event_count: report.forwarded_event_count,
        dropped_event_count: report.dropped_event_count,
        forwarded_target_count: report.forwarded_target_count,
        rate_limited_count: report.rate_limited_count,
        report: Some(report),
    }
}

fn create_runtime_packet_io_evidence(
    sources: &DiagnosticExportSources,
) -> DiagnosticRuntimePacketIoEvidence {
    let Some(snapshot) = sources.runtime_snapshot.as_ref() else {
        return DiagnosticRuntimePacketIoEvidence {
            error: sources.runtime_snapshot_error.clone(),
            ..DiagnosticRuntimePacketIoEvidence::default()
        };
    };
    let raw_virtual_packets = json_array(snapshot, "rawVirtualPackets");
    let wintun_runtime = snapshot.get("wintunRuntime");
    DiagnosticRuntimePacketIoEvidence {
        status: snapshot
            .get("status")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        error: sources.runtime_snapshot_error.clone(),
        raw_virtual_packet_count: raw_virtual_packets.len(),
        forwarded_packet_count: json_array(snapshot, "forwardedPackets").len(),
        injected_packet_count: json_array(snapshot, "injectedPackets").len(),
        wintun_received_packet_count: wintun_runtime
            .and_then(|runtime| runtime.get("receivedPackets"))
            .and_then(serde_json::Value::as_array)
            .map(Vec::len)
            .unwrap_or_default(),
        wintun_sent_packet_count: wintun_runtime
            .and_then(|runtime| runtime.get("sentPackets"))
            .and_then(serde_json::Value::as_array)
            .map(Vec::len)
            .unwrap_or_default(),
        wintun_error_count: wintun_runtime
            .and_then(|runtime| runtime.get("errors"))
            .and_then(serde_json::Value::as_array)
            .map(Vec::len)
            .unwrap_or_default(),
        raw_virtual_packets,
    }
}

fn json_array(value: &serde_json::Value, key: &str) -> Vec<serde_json::Value> {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn json_string_array(value: &serde_json::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{evaluate_connection_path, NatCandidate, NatTraversalOffer};

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

    fn offer(peer_id: &str, candidates: Vec<NatCandidate>) -> NatTraversalOffer {
        NatTraversalOffer {
            schema_version: 1,
            room_id: "room_test".to_owned(),
            peer_id: peer_id.to_owned(),
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
    fn diagnostic_bundle_combines_sections_and_network_observation() {
        let connection_path_report = evaluate_connection_path(
            &offer("peer_a", vec![candidate("host", "10.0.0.2:39090", 100)]),
            &offer(
                "peer_b",
                vec![candidate("srflx", "198.51.100.20:44000", 90)],
            ),
            "ok",
        );
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

Rule Name:                            Example TCP 39077
Enabled:                              Yes
Direction:                            In
Profiles:                             Private
RemoteIP:                             10.77.12.0/24
Protocol:                             TCP
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
                runtime_snapshot: Some(serde_json::json!({
                    "status": "ok",
                    "connectionPathReports": [{
                        "source": "nat-bootstrap-remote-peer",
                        "peerId": "peer_b",
                        "bootstrapStatus": "ok",
                        "report": connection_path_report
                    }],
                    "runtimePeerSummaries": [{
                        "peerId": "peer_b",
                        "virtualIp": "10.77.12.3",
                        "endpoint": "198.51.100.20:44000",
                        "selectedPath": "p2p",
                        "connectionPathStatus": "p2p-candidate-ready",
                        "bootstrapStatus": "ok",
                        "connected": true,
                        "latencyMs": 12,
                        "lastSeenAtMs": 123456,
                        "lastSentAtMs": 123460,
                        "bytesSent": 21,
                        "bytesReceived": 34,
                        "heartbeatPacketsSent": 1,
                        "heartbeatAckPacketsReceived": 1,
                        "heartbeatAckPacketsSent": 0,
                        "heartbeatLossPercent": 0.0,
                        "forwardedPacketsSent": 1,
                        "tunnelPacketsReceived": 1
                    }],
                    "rawVirtualPackets": [{
                        "sourceIp": "10.77.12.2",
                        "destinationIp": "10.77.12.255",
                        "protocol": "udp",
                        "payloadBytes": 8
                    }],
                    "forwardedPackets": [{"bytes": 8}],
                    "broadcastForwardReport": {
                        "status": "ok",
                        "event_count": 1,
                        "forwarded_event_count": 1,
                        "dropped_event_count": 0,
                        "forwarded_target_count": 1,
                        "rate_limited_count": 0,
                        "allowed_ports": [39078],
                        "max_packets_per_second": 30,
                        "events": [{
                            "protocol": "udp",
                            "source_ip": "10.77.12.2",
                            "destination_ip": "10.77.12.255",
                            "destination_port": 39078,
                            "forwarded": true,
                            "reason": "room-broadcast-allowed",
                            "target_count": 1,
                            "packet_io_backend": "wintun"
                        }],
                        "next_action": "Broadcast forwarding decisions look healthy."
                    },
                    "injectedPackets": [],
                    "wintunRuntime": {
                        "receivedPackets": [{"protocol": "udp"}],
                        "sentPackets": [{"protocol": "udp"}],
                        "errors": [],
                        "close": {
                            "session_ended": true,
                            "closed": true
                        }
                    },
                    "runtimeCleanupPlan": {
                        "platform": "windows",
                        "dry_run": true,
                        "room_id": "room_test",
                        "local_peer_id": "peer_a",
                        "local_virtual_ip": "10.77.12.2",
                        "adapter_name": "LocalAreaInterconnection",
                        "packet_io_backend": "wintun",
                        "restore_adapter": false,
                        "requires_elevation": false,
                        "process_cleanup_steps": [{
                            "key": "close-tunnel-socket",
                            "status": "automatic",
                            "detail": "Drop tunnel socket."
                        }],
                        "commands": [],
                        "verification_checks": ["Runtime process has exited."],
                        "warnings": []
                    }
                })),
                runtime_snapshot_error: None,
                route_table: DiagnosticTextSource {
                    source: "route-file".to_owned(),
                    raw_output: r#"
IPv4 Route Table
===========================================================================
Active Routes:
Network Destination        Netmask          Gateway       Interface  Metric
"#
                    .to_owned(),
                    error: None,
                },
                netstat_table: DiagnosticTextSource {
                    source: "netstat-file".to_owned(),
                    raw_output: r#"
  Proto  Local Address          Foreign Address        State           PID
  TCP    0.0.0.0:39077          0.0.0.0:0              LISTENING       4242
"#
                    .to_owned(),
                    error: None,
                },
                relay_fallback_plan: None,
                connection_path_report: None,
                relay_fallback_error: None,
            },
        );

        assert_eq!(bundle.schema_version, 18);
        assert_eq!(bundle.status, "created");
        assert_eq!(bundle.adapter_scan.status, "ok");
        assert_eq!(bundle.ping.status, "ok");
        assert_eq!(bundle.packet_io.status, "ok");
        assert_eq!(bundle.broadcast_forward.status, "ok");
        assert_eq!(bundle.broadcast_forward.event_count, 1);
        assert_eq!(bundle.broadcast_forward.forwarded_event_count, 1);
        assert_eq!(bundle.route_scan.status, "ok");
        assert_eq!(bundle.route_scan.route_count, 0);
        assert_eq!(bundle.route_scan.room_route_count, 0);
        assert_eq!(bundle.game_port_scan.status, "ok");
        assert_eq!(bundle.game_port_scan.endpoint_count, 1);
        assert_eq!(bundle.game_port_scan.match_count, 1);
        assert_eq!(bundle.game_readiness.status, "ready");
        assert_eq!(bundle.game_readiness.game_name, "Example Game");
        assert!(bundle
            .game_readiness
            .checks
            .iter()
            .any(|check| check.key == "firewall" && check.status == "ok"));
        assert!(bundle
            .game_readiness
            .checks
            .iter()
            .any(|check| check.key == "runtime-peer:peer_b" && check.status == "ok"));
        assert_eq!(bundle.relay_fallback.status, "ok");
        assert_eq!(bundle.connection_path.status, "ok");
        assert_eq!(bundle.connection_path.source, "runtime-snapshot-report");
        assert_eq!(
            bundle
                .connection_path
                .report
                .as_ref()
                .unwrap()
                .selected_path,
            "p2p"
        );
        assert_eq!(bundle.packet_io.backend.as_deref(), Some("userspace-udp"));
        assert_eq!(bundle.packet_io.runtime.raw_virtual_packet_count, 1);
        assert_eq!(bundle.packet_io.runtime.wintun_received_packet_count, 1);
        assert_eq!(bundle.runtime_cleanup.status, "ok");
        assert_eq!(bundle.runtime_cleanup.process_step_count, 1);
        assert_eq!(bundle.runtime_cleanup.command_count, 0);
        assert_eq!(bundle.runtime_cleanup.check_count, 5);
        assert_eq!(bundle.runtime_cleanup.next_action_count, 0);
        assert_eq!(bundle.runtime_cleanup.route_count, 0);
        assert!(!bundle.runtime_cleanup.requires_elevation);
        assert_eq!(bundle.runtime_cleanup.report.as_ref().unwrap().status, "ok");
        assert_eq!(bundle.runtime_peers.status, "ok");
        assert_eq!(bundle.runtime_peers.peer_count, 1);
        assert_eq!(bundle.runtime_peers.connected_peer_count, 1);
        assert_eq!(bundle.runtime_peers.total_bytes_sent, 21);
        assert_eq!(
            bundle.runtime_peers.summaries[0].selected_path.as_str(),
            "p2p"
        );
        assert_eq!(bundle.runtime_peers.summaries[0].latency_ms, Some(12));
        assert_eq!(
            bundle.runtime_peers.summaries[0].last_seen_at_ms,
            Some(123456)
        );
        assert_eq!(
            bundle.runtime_peers.summaries[0].last_sent_at_ms,
            Some(123460)
        );
        assert_eq!(
            bundle.runtime_peers.summaries[0].heartbeat_ack_packets_received,
            1
        );
        assert_eq!(
            bundle.runtime_peers.summaries[0].heartbeat_loss_percent,
            Some(0.0)
        );
        assert_eq!(
            bundle.runtime_peers.summaries[0]
                .health
                .as_ref()
                .unwrap()
                .status,
            "ok"
        );
        assert!(bundle
            .network_observation
            .checks
            .iter()
            .any(|check| check.key == "runtime-peer:peer_b" && check.status == "ok"));
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
