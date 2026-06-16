mod cli_args;
mod connection_paths;
mod coordination_http;
mod nat_direct;

use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use clap::{CommandFactory, Parser};
use cli_args::{Cli, Command};
use connection_paths::{
    connection_path_peer_id, connection_path_reports_from_bootstrap_outputs,
    connection_path_status_from_bootstrap_status, load_nat_offer_argument,
    load_relay_fallback_for_export, runtime_relay_fallback_summaries,
};
use coordination_http::{
    coordination_http_close, coordination_http_fetch_offers, coordination_http_heartbeat,
    coordination_http_kick, coordination_http_leave, coordination_http_prune,
    coordination_http_publish_offer, coordination_http_room_view,
    load_coordination_store_or_default, run_coordination_http_server,
};
use lai_core::{
    add_room_member, close_room, create_command_execution_preview, create_diagnostic_export_bundle,
    create_game_network_plan, create_invite, create_join_plan, create_p2p_handshake_ack,
    create_p2p_handshake_hello, create_room, create_room_runtime_plan, create_room_session,
    create_runtime_cleanup_report, create_windows_firewall_plan,
    create_windows_virtual_adapter_ensure_report, create_windows_virtual_adapter_plan,
    decode_invite, evaluate_firewall_diagnostics,
    evaluate_game_readiness_with_firewall_and_connection_path, evaluate_network_observations,
    find_game_profile, list_game_profile_summaries, observation_from_expected_rule,
    open_tunnel_payload, parse_game_profile_catalog_json, parse_netsh_adapter_observation,
    parse_netsh_firewall_rules, parse_windows_ping_observation, seal_tunnel_payload,
    udp_forward_summary, AdapterObservation, CommandExecutionRecord, CommandExecutionStatus,
    CompatibilityLevel, DiagnosticExportEnvironment, DiagnosticExportInputs,
    DiagnosticExportSources, DiagnosticSnapshot, DiagnosticTextSource, DiscoveryMode,
    FirewallDiagnosticsReport, FirewallRule, FirewallRuleObservation, GameProfile, Ipv4Subnet,
    NetworkCommand, NetworkObservationSnapshot, P2pHandshakeAck, P2pHandshakeHello,
    PacketCaptureSummary, PacketObservation, RoomRuntimePeer, RoomRuntimePlan, TunnelEnvelope,
    TunnelObservation, TunnelServiceSnapshot, UdpForwardObservation, VirtualUdpPacket,
};
use nat_direct::{
    apply_stun_mapping_candidates_to_offer, apply_upnp_port_mapping_to_offer,
    enrich_offer_with_local_host_candidates, query_stun_like_server, run_nat_hole_punch,
    run_nat_hole_punch_loopback_test, run_nat_p2p_bootstrap, run_stun_like_server,
    UpnpPortMappingReport,
};
use rand::RngCore;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct RuntimePacketIoProbeOptions {
    wintun_adapter_name: String,
    wintun_ring_capacity: u32,
    wintun_probe_receive: bool,
    wintun_receive_attempts: u32,
    wintun_receive_poll_interval_ms: u64,
    wintun_probe_send: bool,
}

#[derive(Clone, Debug)]
struct RuntimeCoordinationMonitor {
    store_path: Option<String>,
    server: Option<String>,
    interval_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
struct RuntimeCoordinationMonitorReport {
    status: String,
    source: String,
    room_id: String,
    peer_id: String,
    peer_present: bool,
    room_present: bool,
    checked_at_ms: u128,
    detail: String,
}

fn main() {
    let handle = std::thread::Builder::new()
        .name("lai-cli-main".to_owned())
        .stack_size(8 * 1024 * 1024)
        .spawn(|| match run_main() {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("error: {err}");
                1
            }
        });
    let exit_code = match handle {
        Ok(handle) => handle.join().unwrap_or(1),
        Err(err) => {
            eprintln!("error: failed to start CLI thread: {err}");
            1
        }
    };
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

fn run_main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let Some(command) = cli.command else {
        Cli::command().print_help()?;
        println!();
        return Ok(());
    };
    match command {
        Command::Init { room_name, host } => {
            let room = create_room(room_name, host, &[])?;
            let invite = create_invite(&room)?;
            println!("{}", serde_json::to_string_pretty(&(room, invite))?);
        }
        Command::Decode { invite } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&decode_invite(&invite)?)?
            );
        }
        Command::Join { invite } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&create_join_plan(&invite, 0)?)?
            );
        }
        Command::RoomSummary {
            room_name,
            host,
            peers,
            close,
        } => {
            let room = create_room(room_name, host, &[])?;
            let mut session = create_room_session(&room, current_epoch_ms())?;
            for (index, peer_name) in peers.iter().enumerate() {
                add_room_member(
                    &mut session,
                    peer_name,
                    format!("peer_{}", index + 1),
                    index as u32,
                    current_epoch_ms(),
                )?;
            }
            if close {
                close_room(&mut session, current_epoch_ms())?;
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "session": session,
                    "summary": session.summary(),
                }))?
            );
        }
        Command::RoomRuntimePlan {
            room_id,
            peer_id,
            virtual_ip,
            bind,
            peers,
            nat_bootstrap_peers,
            game_ports,
            broadcast_ports,
        } => {
            let plan = create_room_runtime_plan(
                room_id,
                peer_id,
                virtual_ip.parse::<Ipv4Addr>()?,
                bind,
                parse_runtime_peers_with_bootstrap(&peers, &nat_bootstrap_peers)?,
                parse_ports(&game_ports)?,
                parse_ports(&broadcast_ports)?,
            );
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        Command::RuntimeCleanupPlan {
            room_id,
            peer_id,
            virtual_ip,
            subnet,
            adapter_name,
            packet_io_backend,
            restore_adapter,
            cleanup_routes,
        } => {
            let plan = lai_core::create_windows_runtime_cleanup_plan_with_routes(
                room_id,
                peer_id,
                virtual_ip.parse::<Ipv4Addr>()?,
                parse_optional_subnet(subnet.as_deref())?,
                adapter_name,
                packet_io_backend,
                restore_adapter,
                cleanup_routes,
            );
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        Command::RuntimeCleanupReport {
            runtime_snapshot,
            cleanup_plan,
            adapter_netsh_output,
            adapter_scan,
            adapter_name,
            route_output,
            route_scan,
        } => {
            let (runtime_snapshot_value, runtime_snapshot_error) =
                load_runtime_snapshot(runtime_snapshot.as_deref());
            if let Some(error) = runtime_snapshot_error {
                return Err(invalid_input(format!(
                    "failed to load runtime snapshot: {error}"
                )));
            }
            let plan = load_runtime_cleanup_plan_for_report(
                cleanup_plan.as_deref(),
                runtime_snapshot_value.as_ref(),
            )?;
            let adapter_source =
                load_adapter_source(&adapter_name, adapter_netsh_output.as_deref(), adapter_scan);
            let adapter_observation = if adapter_source.raw_output.trim().is_empty() {
                None
            } else {
                parse_netsh_adapter_observation(
                    adapter_name.clone(),
                    &adapter_source.raw_output,
                    Some(plan.local_virtual_ip),
                    None,
                )
            };
            let wintun_close = runtime_snapshot_value
                .as_ref()
                .and_then(runtime_wintun_close_report_from_snapshot);
            let route_source = load_route_source(route_output.as_deref(), route_scan);
            let routes = lai_core::parse_windows_ipv4_routes(&route_source.raw_output);
            let report =
                create_runtime_cleanup_report(plan, adapter_observation, routes, wintun_close);
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "adapterSource": adapter_source,
                    "routeSource": route_source,
                    "report": report,
                }))?
            );
        }
        Command::RuntimeCleanupApply {
            runtime_snapshot,
            cleanup_plan,
            adapter_netsh_output,
            adapter_scan,
            adapter_name,
            route_output,
            route_scan,
            yes,
        } => {
            let (runtime_snapshot_value, runtime_snapshot_error) =
                load_runtime_snapshot(runtime_snapshot.as_deref());
            if let Some(error) = runtime_snapshot_error {
                return Err(invalid_input(format!(
                    "failed to load runtime snapshot: {error}"
                )));
            }
            let plan = load_runtime_cleanup_plan_for_report(
                cleanup_plan.as_deref(),
                runtime_snapshot_value.as_ref(),
            )?;
            let unsafe_commands = runtime_cleanup_command_safety_errors(&plan);
            let adapter_source =
                load_adapter_source(&adapter_name, adapter_netsh_output.as_deref(), adapter_scan);
            let adapter_observation = if adapter_source.raw_output.trim().is_empty() {
                None
            } else {
                parse_netsh_adapter_observation(
                    adapter_name.clone(),
                    &adapter_source.raw_output,
                    Some(plan.local_virtual_ip),
                    plan.virtual_subnet,
                )
            };
            let wintun_close = runtime_snapshot_value
                .as_ref()
                .and_then(runtime_wintun_close_report_from_snapshot);
            let route_source = load_route_source(route_output.as_deref(), route_scan);
            let routes = lai_core::parse_windows_ipv4_routes(&route_source.raw_output);
            let report = create_runtime_cleanup_report(
                plan.clone(),
                adapter_observation,
                routes,
                wintun_close,
            );
            let elevated = detect_windows_elevation();
            let preview = create_command_execution_preview(
                &plan.commands,
                plan.requires_elevation,
                yes,
                elevated,
            );
            let command_results = if preview.can_execute_now && unsafe_commands.is_empty() {
                execute_network_commands(&plan.commands)
            } else {
                Vec::new()
            };
            let status = runtime_cleanup_apply_status(&preview, &command_results, &unsafe_commands);
            let next_action = if command_results.is_empty() {
                if unsafe_commands.is_empty() {
                    preview.next_action.clone()
                } else {
                    "Regenerate the cleanup plan with runtime-cleanup-plan or room-runtime-run; unsafe commands were not executed.".to_owned()
                }
            } else if command_results
                .iter()
                .all(|record| record.status == CommandExecutionStatus::Succeeded)
            {
                "Run runtime-cleanup-report with adapter and route scans to verify cleanup."
                    .to_owned()
            } else {
                "Review failed command output, then rerun from an Administrator terminal if needed."
                    .to_owned()
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": status,
                    "adapterSource": adapter_source,
                    "routeSource": route_source,
                    "reportBeforeApply": report,
                    "executionPreview": preview,
                    "commandResults": command_results,
                    "unsafeCommands": unsafe_commands,
                    "nextAction": next_action,
                }))?
            );
        }
        Command::RouteScan {
            route_output,
            route_scan,
            virtual_ip,
            subnet,
        } => {
            let route_source = load_route_source(route_output.as_deref(), route_scan);
            let routes = lai_core::parse_windows_ipv4_routes(&route_source.raw_output);
            let virtual_ip = parse_optional_ipv4(virtual_ip.as_deref())?;
            let subnet = parse_optional_subnet(subnet.as_deref())?;
            let room_routes = routes
                .iter()
                .filter(|route| route_matches_room(route, virtual_ip, subnet))
                .cloned()
                .collect::<Vec<_>>();
            let status = if route_source.error.is_some() {
                "needs-attention"
            } else if routes.is_empty() {
                "no-data"
            } else if room_routes.is_empty() {
                "ok"
            } else {
                "needs-attention"
            };
            let next_action = if route_source.error.is_some() {
                "Run from a Windows terminal where route.exe is available, or provide --route-output."
            } else if routes.is_empty() {
                "Provide route print -4 output after joining or stopping a room."
            } else if room_routes.is_empty() {
                "No room route residue matched the provided virtual IP or subnet."
            } else {
                "Review matched room routes and run runtime-cleanup-apply from an Administrator terminal if they are stale."
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": status,
                    "routeSource": route_source,
                    "routeCount": routes.len(),
                    "roomRouteCount": room_routes.len(),
                    "routes": routes,
                    "roomRoutes": room_routes,
                    "nextAction": next_action,
                }))?
            );
        }
        Command::GamePortScan {
            netstat_output,
            netstat_scan,
            game_name,
            catalog,
            steam_app_id,
            ports,
            protocols,
        } => {
            let netstat_source = load_netstat_source(netstat_output.as_deref(), netstat_scan);
            let endpoints = if netstat_source.error.is_none() {
                lai_core::parse_windows_netstat_ano(&netstat_source.raw_output)
            } else {
                Vec::new()
            };
            let profile = profile_from_catalog_or_args(
                catalog.as_deref(),
                game_name,
                steam_app_id.as_deref(),
                "manual_ports".to_owned(),
                ports,
                "unknown".to_owned(),
            )?;
            let expected_ports = profile.ports;
            let expected_protocols = parse_protocol_filter(&protocols);
            let matches = endpoints
                .iter()
                .filter(|endpoint| {
                    endpoint_matches_game_ports(endpoint, &expected_ports, &expected_protocols)
                })
                .cloned()
                .collect::<Vec<_>>();
            let status = if netstat_source.error.is_some() {
                "needs-attention"
            } else if expected_ports.is_empty() {
                "no-ports"
            } else if matches.is_empty() {
                "missing"
            } else {
                "ok"
            };
            let next_action = if netstat_source.error.is_some() {
                "Run on Windows where netstat.exe is available, or provide --netstat-output."
            } else if expected_ports.is_empty() {
                "Provide --ports with the game's expected LAN ports."
            } else if matches.is_empty() {
                "Start or host the game, then scan again; also check whether the game uses different ports."
            } else {
                "Game port bindings were observed; run network-observe to check traffic, route, adapter and tunnel state."
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": status,
                    "netstatSource": netstat_source,
                    "gameName": profile.game_name,
                    "endpointCount": endpoints.len(),
                    "matchCount": matches.len(),
                    "expectedPorts": expected_ports,
                    "expectedProtocols": expected_protocols,
                    "matches": matches,
                    "nextAction": next_action,
                }))?
            );
        }
        Command::GameReadiness {
            network_report,
            game_plan,
            catalog,
            game_name,
            steam_app_id,
            subnet,
            discovery,
            ports,
            compatibility,
            host_ip,
            local_ip,
            firewall_netsh_output,
            firewall_scan,
            program,
            netstat_output,
            netstat_scan,
            protocols,
            relay_local_offer,
            relay_remote_offer,
            relay_p2p_status,
        } => {
            let network_report: lai_core::NetworkObservationReport =
                serde_json::from_value(load_json_argument(&network_report)?).map_err(|err| {
                    invalid_input(format!("invalid network observation report: {err}"))
                })?;
            let plan = load_or_create_game_plan(
                game_plan.as_deref(),
                catalog.as_deref(),
                game_name,
                steam_app_id.as_deref(),
                subnet,
                discovery,
                ports,
                compatibility,
                host_ip.as_deref(),
                local_ip.as_deref(),
            )?;
            let firewall_source =
                load_firewall_source(firewall_netsh_output.as_deref(), firewall_scan);
            let firewall_report = game_readiness_firewall_report(
                &plan,
                &firewall_source,
                program.as_deref(),
                firewall_scan || firewall_netsh_output.is_some(),
            );
            let expected_ports = game_plan_ports(&plan);
            let expected_protocols = parse_protocol_filter(&protocols);
            let netstat_source = load_netstat_source(netstat_output.as_deref(), netstat_scan);
            let endpoints = if netstat_source.error.is_none() {
                lai_core::parse_windows_netstat_ano(&netstat_source.raw_output)
            } else {
                Vec::new()
            };
            let matches = endpoints
                .iter()
                .filter(|endpoint| {
                    endpoint_matches_game_ports(endpoint, &expected_ports, &expected_protocols)
                })
                .cloned()
                .collect::<Vec<_>>();
            let (_, connection_path_report, connection_path_error) = load_relay_fallback_for_export(
                relay_local_offer.as_deref(),
                relay_remote_offer.as_deref(),
                &relay_p2p_status,
            );
            let report = evaluate_game_readiness_with_firewall_and_connection_path(
                &plan,
                &network_report,
                &matches,
                firewall_report.as_ref(),
                connection_path_report.as_ref(),
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": report.status,
                    "report": report,
                    "networkStatus": network_report.status,
                    "gamePlan": plan,
                    "firewallSource": firewall_source,
                    "firewallReport": firewall_report,
                    "netstatSource": netstat_source,
                    "endpointCount": endpoints.len(),
                    "matchCount": matches.len(),
                    "matches": matches,
                    "connectionPathReport": connection_path_report,
                    "connectionPathError": connection_path_error,
                }))?
            );
        }
        Command::RoomRuntimeRun {
            room_id,
            peer_id,
            virtual_ip,
            bind,
            peers,
            nat_bootstrap_peers,
            nat_bootstrap_remote_peers,
            coordination_store,
            coordination_server,
            coordination_peers,
            game_ports,
            broadcast_ports,
            max_broadcast_packets_per_second,
            key,
            duration_ms,
            observe_file,
            snapshot_out,
            packet_io_backend,
            forward_raw_ipv4,
            self_probe,
            capture_self_probe,
            forward_self_probe,
            inject_self_probe,
            inject_target,
            heartbeat_interval_ms,
            peer_timeout_ms,
            nat_bootstrap_attempts,
            nat_bootstrap_interval_ms,
            nat_bootstrap_timeout_ms,
            nat_bootstrap_stun_server,
            nat_bootstrap_stun_timeout_ms,
            nat_bootstrap_upnp_port_map,
            nat_bootstrap_upnp_timeout_ms,
            nat_bootstrap_upnp_lease_seconds,
            nat_bootstrap_upnp_gateway_location,
            stop_file,
            snapshot_interval_ms,
            coordination_monitor,
            coordination_monitor_interval_ms,
            wintun_adapter_name,
            wintun_ring_capacity,
            wintun_probe_receive,
            wintun_receive_attempts,
            wintun_receive_poll_interval_ms,
            wintun_probe_send,
            wintun_runtime,
        } => {
            let game_ports = parse_ports(&game_ports)?;
            let broadcast_ports = parse_ports(&broadcast_ports)?;
            let local_virtual_ip = virtual_ip.parse::<Ipv4Addr>()?;
            let mut runtime_peers =
                parse_runtime_peers_with_bootstrap(&peers, &nat_bootstrap_peers)?;
            let (mut bootstrapped_peers, nat_bootstrap_results) = run_runtime_nat_bootstraps(
                &nat_bootstrap_remote_peers,
                &room_id,
                &peer_id,
                local_virtual_ip,
                &key,
                &bind,
                nat_bootstrap_attempts,
                nat_bootstrap_interval_ms,
                nat_bootstrap_timeout_ms,
                nat_bootstrap_stun_server.as_deref(),
                nat_bootstrap_stun_timeout_ms,
                nat_bootstrap_upnp_port_map,
                nat_bootstrap_upnp_timeout_ms,
                nat_bootstrap_upnp_lease_seconds,
                nat_bootstrap_upnp_gateway_location.as_deref(),
            )?;
            runtime_peers.append(&mut bootstrapped_peers);
            let (mut coordination_bootstrapped_peers, mut coordination_bootstrap_results) =
                if coordination_peers.is_empty() {
                    (Vec::new(), Vec::new())
                } else {
                    run_runtime_coordination_bootstraps(
                        coordination_store.as_deref(),
                        &coordination_peers,
                        &room_id,
                        &peer_id,
                        local_virtual_ip,
                        &key,
                        &bind,
                        nat_bootstrap_attempts,
                        nat_bootstrap_interval_ms,
                        nat_bootstrap_timeout_ms,
                        nat_bootstrap_stun_server.as_deref(),
                        nat_bootstrap_stun_timeout_ms,
                        nat_bootstrap_upnp_port_map,
                        nat_bootstrap_upnp_timeout_ms,
                        nat_bootstrap_upnp_lease_seconds,
                        nat_bootstrap_upnp_gateway_location.as_deref(),
                    )?
                };
            runtime_peers.append(&mut coordination_bootstrapped_peers);
            let (mut coordination_server_peers, mut coordination_server_results) =
                if coordination_peers.is_empty() {
                    (Vec::new(), Vec::new())
                } else {
                    run_runtime_coordination_server_bootstraps(
                        coordination_server.as_deref(),
                        &coordination_peers,
                        &room_id,
                        &peer_id,
                        local_virtual_ip,
                        &key,
                        &bind,
                        nat_bootstrap_attempts,
                        nat_bootstrap_interval_ms,
                        nat_bootstrap_timeout_ms,
                        nat_bootstrap_stun_server.as_deref(),
                        nat_bootstrap_stun_timeout_ms,
                        nat_bootstrap_upnp_port_map,
                        nat_bootstrap_upnp_timeout_ms,
                        nat_bootstrap_upnp_lease_seconds,
                        nat_bootstrap_upnp_gateway_location.as_deref(),
                    )?
                };
            runtime_peers.append(&mut coordination_server_peers);
            let connection_path_reports = connection_path_reports_from_bootstrap_outputs(
                &nat_bootstrap_results,
                &coordination_bootstrap_results,
                &coordination_server_results,
            )?;
            let plan = create_room_runtime_plan(
                room_id,
                peer_id,
                local_virtual_ip,
                bind,
                runtime_peers,
                game_ports.clone(),
                broadcast_ports.clone(),
            );
            let mut result = run_room_runtime(
                &plan,
                &key,
                duration_ms,
                observe_file.as_deref(),
                snapshot_out.as_deref(),
                &packet_io_backend,
                forward_raw_ipv4,
                self_probe,
                capture_self_probe,
                forward_self_probe,
                inject_self_probe,
                inject_target.as_deref(),
                heartbeat_interval_ms,
                peer_timeout_ms,
                stop_file.as_deref(),
                snapshot_interval_ms,
                coordination_monitor.then(|| RuntimeCoordinationMonitor {
                    store_path: coordination_store.clone(),
                    server: coordination_server.clone(),
                    interval_ms: coordination_monitor_interval_ms,
                }),
                &RuntimePacketIoProbeOptions {
                    wintun_adapter_name,
                    wintun_ring_capacity,
                    wintun_probe_receive,
                    wintun_receive_attempts,
                    wintun_receive_poll_interval_ms,
                    wintun_probe_send,
                },
                wintun_runtime,
                broadcast_ports.clone(),
                game_ports.clone(),
                max_broadcast_packets_per_second,
            )?;
            result["natBootstrapResults"] = serde_json::Value::Array(nat_bootstrap_results);
            coordination_bootstrap_results.append(&mut coordination_server_results);
            result["coordinationBootstrapResults"] =
                serde_json::Value::Array(std::mem::take(&mut coordination_bootstrap_results));
            result["connectionPathReports"] =
                serde_json::Value::Array(connection_path_reports.clone());
            result["runtimeRelayFallbackSummaries"] = serde_json::Value::Array(
                runtime_relay_fallback_summaries(&connection_path_reports),
            );
            let actual_tunnel_endpoint = result
                .get("actualTunnelEndpoint")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let runtime_peer_summary_values = runtime_peer_summaries(
                &plan,
                &connection_path_reports,
                json_array_values(&result, "tunnelPackets").as_slice(),
                json_array_values(&result, "forwardedPackets").as_slice(),
                json_array_values(&result, "heartbeatPackets").as_slice(),
                json_array_values(&result, "heartbeatAckPackets").as_slice(),
                result
                    .get("tunnelServiceSnapshot")
                    .and_then(|snapshot| snapshot.get("connection_path"))
                    .and_then(serde_json::Value::as_str),
                actual_tunnel_endpoint.as_deref(),
            );
            refresh_runtime_network_observation(
                &mut result,
                &plan,
                runtime_peer_summary_values,
                broadcast_ports,
                game_ports,
            )?;
            if let Some(path) = snapshot_out.as_deref() {
                write_json_file(path, &result)?;
            }
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Diagnose { p2p, firewall } => {
            let report = lai_core::evaluate_diagnostics(DiagnosticSnapshot {
                p2p,
                firewall,
                ..DiagnosticSnapshot::default()
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::GamePlan {
            game_name,
            subnet,
            discovery,
            ports,
            compatibility,
            host_ip,
            local_ip,
        } => {
            let profile = profile_from_args(game_name, discovery, ports, compatibility)?;
            let subnet = subnet.parse::<Ipv4Subnet>()?;
            let plan = create_game_network_plan(
                &profile,
                subnet,
                parse_optional_ipv4(host_ip.as_deref())?,
                parse_optional_ipv4(local_ip.as_deref())?,
                30,
            );
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        Command::GameProfilePlan {
            catalog,
            game_name,
            steam_app_id,
            subnet,
            host_ip,
            local_ip,
            max_broadcast_packets_per_second,
        } => {
            if game_name.as_deref().unwrap_or_default().trim().is_empty()
                && steam_app_id
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
            {
                return Err(invalid_input(
                    "game-profile-plan requires --game-name or --steam-app-id".to_owned(),
                ));
            }
            let catalog_text = fs::read_to_string(catalog)?;
            let catalog = parse_game_profile_catalog_json(&catalog_text)?;
            let matched =
                find_game_profile(&catalog, game_name.as_deref(), steam_app_id.as_deref())
                    .ok_or_else(|| {
                        invalid_input(format!(
                            "game profile not found for game_name={:?}, steam_app_id={:?}",
                            game_name, steam_app_id
                        ))
                    })?;
            let subnet = subnet.parse::<Ipv4Subnet>()?;
            let plan = create_game_network_plan(
                &matched.profile,
                subnet,
                parse_optional_ipv4(host_ip.as_deref())?,
                parse_optional_ipv4(local_ip.as_deref())?,
                max_broadcast_packets_per_second,
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "ok",
                    "matched_by": matched.matched_by,
                    "profile": matched.profile,
                    "plan": plan,
                }))?
            );
        }
        Command::GameProfileList { catalog, query } => {
            let catalog_text = fs::read_to_string(catalog)?;
            let catalog = parse_game_profile_catalog_json(&catalog_text)?;
            let profiles = list_game_profile_summaries(&catalog, query.as_deref());
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "ok",
                    "total_count": catalog.profiles.len(),
                    "matched_count": profiles.len(),
                    "profiles": profiles,
                }))?
            );
        }
        Command::FirewallPlan {
            game_name,
            catalog,
            steam_app_id,
            subnet,
            discovery,
            ports,
            compatibility,
            program,
        } => {
            let profile = profile_from_catalog_or_args(
                catalog.as_deref(),
                game_name,
                steam_app_id.as_deref(),
                discovery,
                ports,
                compatibility,
            )?;
            let subnet = subnet.parse::<Ipv4Subnet>()?;
            let network_plan = create_game_network_plan(&profile, subnet, None, None, 30);
            let firewall_plan = create_windows_firewall_plan(
                &network_plan.firewall_rules,
                "LocalAreaInterconnection",
                program,
            );
            println!("{}", serde_json::to_string_pretty(&firewall_plan)?);
        }
        Command::FirewallApply {
            game_name,
            catalog,
            steam_app_id,
            subnet,
            discovery,
            ports,
            compatibility,
            program,
            remote_scope,
            yes,
        } => {
            let profile = profile_from_catalog_or_args(
                catalog.as_deref(),
                game_name,
                steam_app_id.as_deref(),
                discovery,
                ports,
                compatibility,
            )?;
            let subnet = subnet.parse::<Ipv4Subnet>()?;
            let mut network_plan = create_game_network_plan(&profile, subnet, None, None, 30);
            if let Some(remote_scope) = remote_scope {
                for rule in &mut network_plan.firewall_rules {
                    rule.remote_scope = remote_scope.clone();
                }
            }
            let firewall_plan = create_windows_firewall_plan(
                &network_plan.firewall_rules,
                "LocalAreaInterconnection",
                program,
            );
            let elevated = detect_windows_elevation();
            let preview = create_command_execution_preview(
                &firewall_commands_as_network_commands(&firewall_plan.commands),
                firewall_plan.requires_elevation,
                yes,
                elevated,
            );
            let command_results = if preview.can_execute_now {
                execute_firewall_commands(&firewall_plan.commands)
            } else {
                Vec::new()
            };
            let status = if preview.can_execute_now
                && command_results
                    .iter()
                    .all(|record| record.status == CommandExecutionStatus::Succeeded)
            {
                "applied".to_owned()
            } else if preview.can_execute_now {
                "failed".to_owned()
            } else if !yes {
                "needs-confirmation".to_owned()
            } else if firewall_plan.requires_elevation && elevated != Some(true) {
                "needs-elevation".to_owned()
            } else {
                "planned".to_owned()
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": status,
                    "platform": firewall_plan.platform,
                    "gameName": profile.game_name,
                    "requiresElevation": firewall_plan.requires_elevation,
                    "confirmed": yes,
                    "elevated": elevated,
                    "executionPreview": preview,
                    "commandResults": command_results,
                    "warnings": firewall_plan.warnings,
                    "nextAction": if status == "applied" {
                        "Run firewall diagnostics or game readiness again to verify the inbound rules."
                    } else {
                        "Review the firewall commands, approve the Administrator prompt, then run firewall diagnostics again."
                    },
                }))?
            );
        }
        Command::FirewallDiagnose {
            game_name,
            catalog,
            steam_app_id,
            subnet,
            discovery,
            ports,
            compatibility,
            observed,
            netsh_output,
            program,
        } => {
            let profile = profile_from_catalog_or_args(
                catalog.as_deref(),
                game_name,
                steam_app_id.as_deref(),
                discovery,
                ports,
                compatibility,
            )?;
            let subnet = subnet.parse::<Ipv4Subnet>()?;
            let network_plan = create_game_network_plan(&profile, subnet, None, None, 30);
            let observed_rules = if let Some(path) = netsh_output {
                parse_netsh_firewall_rules(&fs::read_to_string(path)?)
            } else {
                observed_firewall_rules(&network_plan.firewall_rules, &observed, program.clone())?
            };
            let report = evaluate_firewall_diagnostics(
                &network_plan.firewall_rules,
                &observed_rules,
                program.as_deref(),
            );
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::AdapterPlan {
            adapter_name,
            subnet,
            ip,
            mtu,
            metric,
        } => {
            let subnet = subnet.parse::<Ipv4Subnet>()?;
            let ip = ip.parse::<Ipv4Addr>()?;
            let plan = create_windows_virtual_adapter_plan(adapter_name, subnet, ip, mtu, metric);
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        Command::AdapterApply {
            adapter_name,
            subnet,
            ip,
            mtu,
            metric,
            yes,
        } => {
            let subnet = subnet.parse::<Ipv4Subnet>()?;
            let ip = ip.parse::<Ipv4Addr>()?;
            let plan = create_windows_virtual_adapter_plan(adapter_name, subnet, ip, mtu, metric);
            let elevated = detect_windows_elevation();
            let preview = create_command_execution_preview(
                &plan.commands,
                plan.requires_elevation,
                yes,
                elevated,
            );
            if !preview.can_execute_now {
                println!("{}", serde_json::to_string_pretty(&preview)?);
            } else {
                let records = execute_network_commands(&plan.commands);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "platform": "windows",
                        "adapterName": plan.adapter_name,
                        "requiresElevation": plan.requires_elevation,
                        "confirmed": yes,
                        "elevated": elevated,
                        "status": if records.iter().all(|record| record.status == CommandExecutionStatus::Succeeded) { "ok" } else { "failed" },
                        "commands": records,
                        "nextAction": "Run adapter/network diagnostics to verify the assigned virtual IP, MTU and interface metric."
                    }))?
                );
            }
        }
        Command::AdapterEnsure {
            adapter_name,
            subnet,
            ip,
            mtu,
            metric,
            adapter_netsh_output,
            adapter_scan,
            yes,
        } => {
            let subnet = subnet.parse::<Ipv4Subnet>()?;
            let ip = ip.parse::<Ipv4Addr>()?;
            let plan =
                create_windows_virtual_adapter_plan(adapter_name.clone(), subnet, ip, mtu, metric);
            let adapter_source =
                load_adapter_source(&adapter_name, adapter_netsh_output.as_deref(), adapter_scan);
            let observation = if adapter_source.raw_output.trim().is_empty() {
                None
            } else {
                parse_netsh_adapter_observation(
                    adapter_name.clone(),
                    &adapter_source.raw_output,
                    Some(ip),
                    Some(subnet),
                )
            };
            let elevated = detect_windows_elevation();
            let report = create_windows_virtual_adapter_ensure_report(
                plan.clone(),
                observation,
                yes,
                elevated,
            );
            let command_results = if report.execution_preview.can_execute_now {
                execute_network_commands(&plan.commands)
            } else {
                Vec::new()
            };
            let executed = !command_results.is_empty();
            let output_status = if executed
                && command_results
                    .iter()
                    .all(|record| record.status == CommandExecutionStatus::Succeeded)
            {
                "applied".to_owned()
            } else {
                report.status.clone()
            };
            let next_action = if executed {
                "Run adapter-ensure again or run network diagnostics to verify the virtual adapter."
                    .to_owned()
            } else {
                report.next_action.clone()
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": output_status,
                    "ready": report.ready,
                    "adapterName": report.adapter_name,
                    "adapterSource": adapter_source,
                    "observation": report.observation,
                    "checks": report.checks,
                    "executionPreview": report.execution_preview,
                    "commandResults": command_results,
                    "nextAction": next_action,
                }))?
            );
        }
        Command::VirtualPacketPlan {
            adapter_name,
            backend,
            mtu,
        } => {
            let plan = lai_core::create_virtual_packet_io_plan(adapter_name, backend, mtu);
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        Command::VirtualPacketBuildUdp {
            source_ip,
            destination_ip,
            source_port,
            destination_port,
            message,
            ttl,
        } => {
            let packet = VirtualUdpPacket {
                source_ip: source_ip.parse::<Ipv4Addr>()?,
                destination_ip: destination_ip.parse::<Ipv4Addr>()?,
                source_port,
                destination_port,
                payload: message.as_bytes().to_vec(),
                broadcast: is_broadcast_destination(&destination_ip)?,
            };
            let bytes = lai_core::build_ipv4_udp_packet(&packet, ttl).map_err(invalid_input)?;
            let observation = lai_core::udp_observation_from_virtual_packet(&packet);
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "ok",
                    "packet": packet,
                    "packetBytes": bytes.len(),
                    "packetBase64": STANDARD_NO_PAD.encode(&bytes),
                    "packetObservationLine": lai_core::packet_observation_line_from_udp_forward(&observation),
                }))?
            );
        }
        Command::VirtualPacketBuildTcp {
            source_ip,
            destination_ip,
            source_port,
            destination_port,
            message,
            flags,
            ttl,
        } => {
            let packet = lai_core::VirtualTcpPacket {
                source_ip: source_ip.parse::<Ipv4Addr>()?,
                destination_ip: destination_ip.parse::<Ipv4Addr>()?,
                source_port,
                destination_port,
                payload: message.as_bytes().to_vec(),
                flags,
            };
            let bytes = lai_core::build_ipv4_tcp_packet(&packet, ttl).map_err(invalid_input)?;
            let observation = lai_core::tcp_observation_from_virtual_packet(&packet);
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "ok",
                    "packet": packet,
                    "packetBytes": bytes.len(),
                    "packetBase64": STANDARD_NO_PAD.encode(&bytes),
                    "packetObservationLine": lai_core::packet_observation_line_from_transport("tcp", &observation),
                }))?
            );
        }
        Command::VirtualPacketParse { packet_base64 } => {
            let bytes = STANDARD_NO_PAD
                .decode(packet_base64.as_bytes())
                .map_err(|err| invalid_input(format!("invalid virtual packet base64: {err}")))?;
            let packet = lai_core::parse_ipv4_udp_packet(&bytes).map_err(invalid_input)?;
            let observation = lai_core::udp_observation_from_virtual_packet(&packet);
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "ok",
                    "packet": packet,
                    "packetBytes": bytes.len(),
                    "packetObservationLine": lai_core::packet_observation_line_from_udp_forward(&observation),
                }))?
            );
        }
        Command::VirtualPacketParseSummary { packet_base64 } => {
            let bytes = STANDARD_NO_PAD
                .decode(packet_base64.as_bytes())
                .map_err(|err| invalid_input(format!("invalid virtual packet base64: {err}")))?;
            let summary = lai_core::parse_ipv4_packet_summary(&bytes).map_err(invalid_input)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "ok",
                    "summary": summary,
                    "packetBytes": bytes.len(),
                }))?
            );
        }
        Command::VirtualPacketLoopbackTest {
            source_ip,
            destination_ip,
            source_port,
            destination_port,
            message,
        } => {
            let packet = VirtualUdpPacket {
                source_ip: source_ip.parse::<Ipv4Addr>()?,
                destination_ip: destination_ip.parse::<Ipv4Addr>()?,
                source_port,
                destination_port,
                payload: message.as_bytes().to_vec(),
                broadcast: is_broadcast_destination(&destination_ip)?,
            };
            let bytes = lai_core::build_ipv4_udp_packet(&packet, 64).map_err(invalid_input)?;
            let parsed = lai_core::parse_ipv4_udp_packet(&bytes).map_err(invalid_input)?;
            let observation = lai_core::udp_observation_from_virtual_packet(&parsed);
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": if parsed == packet { "ok" } else { "mismatch" },
                    "packet": parsed,
                    "packetBytes": bytes.len(),
                    "packetBase64": STANDARD_NO_PAD.encode(&bytes),
                    "packetObservationLine": lai_core::packet_observation_line_from_udp_forward(&observation),
                }))?
            );
        }
        Command::TunnelSeal {
            key,
            packet_kind,
            sequence,
            message,
        } => {
            let envelope = seal_tunnel_payload(
                &key,
                packet_kind,
                sequence,
                current_epoch_ms(),
                message.as_bytes(),
            )?;
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        }
        Command::TunnelOpen { key, envelope } => {
            let envelope: TunnelEnvelope = serde_json::from_str(&envelope)?;
            let payload = open_tunnel_payload(&key, &envelope)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "metadata": payload.metadata,
                    "message": String::from_utf8_lossy(&payload.plaintext),
                    "bytes": payload.plaintext.len(),
                }))?
            );
        }
        Command::TunnelLoopbackTest {
            bind,
            key,
            message,
            timeout_ms,
        } => {
            let result = run_tunnel_loopback_test(&bind, &key, &message, timeout_ms)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::TunnelListen {
            bind,
            key,
            max_packets,
            timeout_ms,
            echo,
        } => {
            let result = run_tunnel_listener(&bind, &key, max_packets, timeout_ms, echo)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::TunnelSend {
            bind,
            peer,
            key,
            message,
            timeout_ms,
            wait_reply,
        } => {
            let result = run_tunnel_send(&bind, &peer, &key, &message, timeout_ms, wait_reply)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::RelayUdpServer {
            bind,
            key,
            room_id,
            allowed_peers,
            max_packets,
            timeout_ms,
        } => {
            let result = run_relay_udp_server(
                &bind,
                &key,
                &room_id,
                &allowed_peers,
                max_packets,
                timeout_ms,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::RelayUdpLoopbackTest {
            bind,
            key,
            room_id,
            message,
            timeout_ms,
        } => {
            let result = run_relay_udp_loopback_test(&bind, &key, &room_id, &message, timeout_ms)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::P2pHandshakeLoopbackTest {
            bind,
            room_id,
            peer_id,
            responder_peer_id,
            virtual_ip,
            key,
            timeout_ms,
        } => {
            let result = run_p2p_handshake_loopback_test(
                &bind,
                &room_id,
                &peer_id,
                &responder_peer_id,
                virtual_ip.parse::<Ipv4Addr>()?,
                &key,
                timeout_ms,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::P2pHandshakeListen {
            bind,
            key,
            responder_peer_id,
            max_packets,
            timeout_ms,
        } => {
            let result = run_p2p_handshake_listener(
                &bind,
                &key,
                &responder_peer_id,
                max_packets,
                timeout_ms,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::P2pHandshakeSend {
            bind,
            peer,
            room_id,
            peer_id,
            virtual_ip,
            key,
            timeout_ms,
        } => {
            let result = run_p2p_handshake_send(
                &bind,
                &peer,
                &room_id,
                &peer_id,
                virtual_ip.parse::<Ipv4Addr>()?,
                &key,
                timeout_ms,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::NatCandidates {
            room_id,
            peer_id,
            virtual_ip,
            bind,
            observed_endpoint,
            stun_server,
            stun_timeout_ms,
            relay_endpoints,
            upnp_port_map,
            upnp_timeout_ms,
            upnp_lease_seconds,
            upnp_gateway_location,
            nonce,
        } => {
            let socket = UdpSocket::bind(&bind)?;
            let local_endpoint = socket.local_addr()?;
            let observed_endpoint = observed_endpoint
                .as_deref()
                .map(str::parse::<SocketAddr>)
                .transpose()?;
            let relay_endpoints = relay_endpoints
                .iter()
                .map(|endpoint| endpoint.parse::<SocketAddr>())
                .collect::<Result<Vec<_>, _>>()?;
            let mut offer = lai_core::create_nat_traversal_offer(
                &room_id,
                &peer_id,
                nonce.unwrap_or_else(random_nonce),
                current_epoch_ms(),
                local_endpoint,
                observed_endpoint,
                relay_endpoints,
            );
            offer.virtual_ip = virtual_ip
                .as_deref()
                .map(str::parse::<Ipv4Addr>)
                .transpose()?;
            enrich_offer_with_local_host_candidates(&mut offer, &socket)?;
            let stun_mapping = apply_stun_mapping_candidates_to_offer(
                &mut offer,
                &socket,
                stun_server.as_deref(),
                stun_timeout_ms,
            );
            let upnp_mapping = if upnp_port_map {
                apply_upnp_port_mapping_to_offer(
                    &mut offer,
                    &socket,
                    upnp_timeout_ms,
                    upnp_lease_seconds,
                    upnp_gateway_location.as_deref(),
                )
            } else {
                UpnpPortMappingReport::disabled()
            };
            let message = lai_core::create_coordination_message(
                "candidate-offer",
                room_id,
                peer_id,
                1,
                current_epoch_ms(),
                Some(offer.clone()),
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "ok",
                    "offer": offer,
                    "coordinationMessage": message,
                    "stunMapping": stun_mapping,
                    "upnpPortMapping": upnp_mapping,
                }))?
            );
        }
        Command::NatPlan {
            local_offer,
            remote_offer,
            attempts,
            interval_ms,
        } => {
            let local = load_nat_offer_argument(&local_offer)?;
            let remote = load_nat_offer_argument(&remote_offer)?;
            let plan = lai_core::create_nat_punch_plan(&local, &remote, attempts, interval_ms);
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        Command::RelayFallbackPlan {
            local_offer,
            remote_offer,
            p2p_status,
        } => {
            let local = load_nat_offer_argument(&local_offer)?;
            let remote = load_nat_offer_argument(&remote_offer)?;
            let plan = lai_core::create_relay_fallback_plan(&local, &remote, p2p_status);
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        Command::ConnectionPathPlan {
            local_offer,
            remote_offer,
            p2p_status,
        } => {
            let local = load_nat_offer_argument(&local_offer)?;
            let remote = load_nat_offer_argument(&remote_offer)?;
            let report = lai_core::evaluate_connection_path(&local, &remote, p2p_status);
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::NatHolePunch {
            room_id,
            peer_id,
            bind,
            remote_offer,
            observed_endpoint,
            stun_server,
            stun_timeout_ms,
            relay_endpoints,
            upnp_port_map,
            upnp_timeout_ms,
            upnp_lease_seconds,
            upnp_gateway_location,
            attempts,
            interval_ms,
            receive_timeout_ms,
            message,
        } => {
            let remote = load_nat_offer_argument(&remote_offer)?;
            let observed_endpoint = observed_endpoint
                .as_deref()
                .map(str::parse::<SocketAddr>)
                .transpose()?;
            let relay_endpoints = relay_endpoints
                .iter()
                .map(|endpoint| endpoint.parse::<SocketAddr>())
                .collect::<Result<Vec<_>, _>>()?;
            let result = run_nat_hole_punch(
                &room_id,
                &peer_id,
                &bind,
                &remote,
                observed_endpoint,
                stun_server.as_deref(),
                stun_timeout_ms,
                upnp_port_map,
                upnp_timeout_ms,
                upnp_lease_seconds,
                upnp_gateway_location.as_deref(),
                relay_endpoints,
                attempts,
                interval_ms,
                receive_timeout_ms,
                &message,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::NatP2pBootstrap {
            room_id,
            peer_id,
            virtual_ip,
            key,
            bind,
            remote_offer,
            observed_endpoint,
            stun_server,
            stun_timeout_ms,
            relay_endpoints,
            upnp_port_map,
            upnp_timeout_ms,
            upnp_lease_seconds,
            upnp_gateway_location,
            punch_attempts,
            punch_interval_ms,
            handshake_timeout_ms,
        } => {
            let remote = load_nat_offer_argument(&remote_offer)?;
            let observed_endpoint = observed_endpoint
                .as_deref()
                .map(str::parse::<SocketAddr>)
                .transpose()?;
            let relay_endpoints = relay_endpoints
                .iter()
                .map(|endpoint| endpoint.parse::<SocketAddr>())
                .collect::<Result<Vec<_>, _>>()?;
            let result = run_nat_p2p_bootstrap(
                &room_id,
                &peer_id,
                virtual_ip.parse::<Ipv4Addr>()?,
                &key,
                &bind,
                &remote,
                observed_endpoint,
                stun_server.as_deref(),
                stun_timeout_ms,
                upnp_port_map,
                upnp_timeout_ms,
                upnp_lease_seconds,
                upnp_gateway_location.as_deref(),
                relay_endpoints,
                punch_attempts,
                punch_interval_ms,
                handshake_timeout_ms,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::NatHolePunchLoopbackTest {
            room_id,
            peer_a,
            peer_b,
            attempts,
            interval_ms,
            message,
        } => {
            let result = run_nat_hole_punch_loopback_test(
                &room_id,
                &peer_a,
                &peer_b,
                attempts,
                interval_ms,
                &message,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::StunLikeServe {
            bind,
            max_requests,
            timeout_ms,
        } => {
            let result = run_stun_like_server(&bind, max_requests, timeout_ms)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::StunLikeQuery {
            bind,
            server,
            timeout_ms,
        } => {
            let socket = UdpSocket::bind(&bind)?;
            let result = query_stun_like_server(&socket, &server, timeout_ms)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::CoordinationStoreInit { out } => {
            let store = lai_core::create_coordination_store();
            write_json_file(&out, &store)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "ok",
                    "store": out,
                    "schemaVersion": store.schema_version,
                }))?
            );
        }
        Command::CoordinationOfferPublish {
            store,
            offer,
            ttl_ms,
        } => {
            let mut coordination_store = load_coordination_store_or_default(&store)?;
            let offer = load_nat_offer_argument(&offer)?;
            let update = lai_core::publish_coordination_offer(
                &mut coordination_store,
                offer,
                current_epoch_ms(),
                ttl_ms,
            );
            write_json_file(&store, &coordination_store)?;
            println!("{}", serde_json::to_string_pretty(&update)?);
        }
        Command::CoordinationOfferFetch {
            store,
            room_id,
            peer_id,
        } => {
            let mut coordination_store = load_coordination_store_or_default(&store)?;
            let result = lai_core::fetch_coordination_offers(
                &mut coordination_store,
                room_id,
                peer_id,
                current_epoch_ms(),
            );
            write_json_file(&store, &coordination_store)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::CoordinationHeartbeat {
            store,
            room_id,
            peer_id,
            ttl_ms,
        } => {
            let mut coordination_store = load_coordination_store_or_default(&store)?;
            let update = lai_core::heartbeat_coordination_peer(
                &mut coordination_store,
                room_id,
                peer_id,
                current_epoch_ms(),
                ttl_ms,
            );
            write_json_file(&store, &coordination_store)?;
            println!("{}", serde_json::to_string_pretty(&update)?);
        }
        Command::CoordinationLeave {
            store,
            room_id,
            peer_id,
        } => {
            let mut coordination_store = load_coordination_store_or_default(&store)?;
            let report = lai_core::leave_coordination_room(
                &mut coordination_store,
                room_id,
                peer_id,
                current_epoch_ms(),
            );
            write_json_file(&store, &coordination_store)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::CoordinationKick {
            store,
            room_id,
            peer_id,
            kicked_by,
        } => {
            let mut coordination_store = load_coordination_store_or_default(&store)?;
            let report = lai_core::kick_coordination_peer(
                &mut coordination_store,
                room_id,
                peer_id,
                kicked_by,
                current_epoch_ms(),
            );
            write_json_file(&store, &coordination_store)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::CoordinationClose {
            store,
            room_id,
            closed_by,
        } => {
            let mut coordination_store = load_coordination_store_or_default(&store)?;
            let report = if let Some(closed_by) = closed_by {
                lai_core::close_coordination_room_by_peer(
                    &mut coordination_store,
                    room_id,
                    closed_by,
                )
            } else {
                lai_core::close_coordination_room(&mut coordination_store, room_id)
            };
            write_json_file(&store, &coordination_store)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::CoordinationRoomView {
            store,
            room_id,
            peer_id,
            subnet,
        } => {
            let coordination_store = load_coordination_store_or_default(&store)?;
            let view = lai_core::coordination_room_view(
                &coordination_store,
                room_id,
                peer_id,
                subnet.parse::<Ipv4Subnet>()?,
                current_epoch_ms(),
            );
            println!("{}", serde_json::to_string_pretty(&view)?);
        }
        Command::CoordinationPrune { store } => {
            let mut coordination_store = load_coordination_store_or_default(&store)?;
            let report = lai_core::prune_expired_coordination_peers(
                &mut coordination_store,
                current_epoch_ms(),
            );
            write_json_file(&store, &coordination_store)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::CoordinationHttpServe {
            bind,
            store,
            max_requests,
            request_timeout_ms,
        } => {
            let result =
                run_coordination_http_server(&bind, &store, max_requests, request_timeout_ms)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::CoordinationHttpOfferPublish {
            server,
            offer,
            ttl_ms,
        } => {
            let offer = load_nat_offer_argument(&offer)?;
            let result = coordination_http_publish_offer(&server, &offer, ttl_ms)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::CoordinationHttpOfferFetch {
            server,
            room_id,
            peer_id,
        } => {
            let result = coordination_http_fetch_offers(&server, &room_id, &peer_id)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::CoordinationHttpRoomView {
            server,
            room_id,
            peer_id,
            subnet,
        } => {
            let result = coordination_http_room_view(
                &server,
                &room_id,
                &peer_id,
                subnet.parse::<Ipv4Subnet>()?,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::CoordinationHttpHeartbeat {
            server,
            room_id,
            peer_id,
            ttl_ms,
        } => {
            let result = coordination_http_heartbeat(&server, &room_id, &peer_id, ttl_ms)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::CoordinationHttpLeave {
            server,
            room_id,
            peer_id,
        } => {
            let result = coordination_http_leave(&server, &room_id, &peer_id)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::CoordinationHttpKick {
            server,
            room_id,
            peer_id,
            kicked_by,
        } => {
            let result = coordination_http_kick(&server, &room_id, &peer_id, &kicked_by)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::CoordinationHttpClose {
            server,
            room_id,
            closed_by,
        } => {
            let result = coordination_http_close(&server, &room_id, closed_by.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::CoordinationHttpPrune { server } => {
            let result = coordination_http_prune(&server)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::UdpForward {
            listen,
            forward,
            max_packets,
            timeout_ms,
            observe_file,
            broadcast,
        } => {
            let result = run_udp_forwarder(
                &listen,
                &forward,
                max_packets,
                timeout_ms,
                observe_file.as_deref(),
                broadcast,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::UdpForwardLoopbackTest {
            message,
            observe_file,
        } => {
            let result = run_udp_forward_loopback_test(&message, observe_file.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::UdpCapture {
            listen,
            max_packets,
            timeout_ms,
            observe_file,
            broadcast,
        } => {
            let result = run_udp_capture(
                &listen,
                max_packets,
                timeout_ms,
                observe_file.as_deref(),
                broadcast,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::UdpCaptureLoopbackTest {
            message,
            observe_file,
        } => {
            let result = run_udp_capture_loopback_test(&message, observe_file.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::UdpLoopbackTest {
            port,
            message,
            timeout_ms,
            observe_file,
        } => {
            let result =
                run_udp_loopback_test(port, &message, timeout_ms, observe_file.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::UdpBroadcastTest {
            port,
            message,
            timeout_ms,
            observe_file,
        } => {
            let result =
                run_udp_broadcast_test(port, &message, timeout_ms, observe_file.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::TcpLoopbackTest {
            port,
            message,
            timeout_ms,
            observe_file,
        } => {
            let result =
                run_tcp_loopback_test(port, &message, timeout_ms, observe_file.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::NetworkObserve {
            adapter_name,
            adapter_enabled,
            expected_ip,
            assigned_ip,
            subnet,
            adapter_netsh_output,
            adapter_scan,
            tunnel_state,
            connected_peers,
            expected_peers,
            latency_ms,
            packet_loss_percent,
            connection_path,
            ping_test,
            ping_output,
            broadcast_ports,
            game_ports,
            packets,
            packet_observations,
            runtime_snapshot,
            route_output,
            route_scan,
        } => {
            let expected_ip = parse_optional_ipv4(expected_ip.as_deref())?;
            let expected_subnet = parse_optional_subnet(subnet.as_deref())?;
            let explicit_adapter_name = adapter_name.clone();
            let adapter_name_value =
                adapter_name.unwrap_or_else(|| "LocalAreaInterconnection".to_owned());
            let adapter_source = load_adapter_source(
                &adapter_name_value,
                adapter_netsh_output.as_deref(),
                adapter_scan,
            );
            let adapter = if !adapter_source.raw_output.trim().is_empty() {
                parse_netsh_adapter_observation(
                    adapter_name_value.clone(),
                    &adapter_source.raw_output,
                    expected_ip,
                    expected_subnet,
                )
            } else if explicit_adapter_name.is_none() {
                None
            } else {
                Some(AdapterObservation {
                    adapter_name: adapter_name_value,
                    enabled: adapter_enabled,
                    expected_ip,
                    assigned_ip: parse_optional_ipv4(assigned_ip.as_deref())?,
                    virtual_subnet: expected_subnet,
                    mtu: None,
                    interface_metric: None,
                })
            };
            let mut packet_observations_data = if let Some(path) = packet_observations {
                lai_core::parse_packet_observation_lines(&fs::read_to_string(path)?)?
            } else {
                Vec::new()
            };
            packet_observations_data.extend(parse_packet_observations(&packets)?);
            let (runtime_snapshot_value, runtime_snapshot_error) =
                load_runtime_snapshot(runtime_snapshot.as_deref());
            if let Some(error) = runtime_snapshot_error.as_ref() {
                return Err(
                    invalid_input(format!("failed to load runtime snapshot: {error}")).into(),
                );
            }
            let runtime_tunnel = runtime_snapshot_value
                .as_ref()
                .and_then(|snapshot| snapshot.get("tunnelServiceSnapshot").cloned())
                .map(serde_json::from_value::<TunnelServiceSnapshot>)
                .transpose()?
                .map(|snapshot| lai_core::tunnel_observation_from_service(&snapshot));
            let runtime_capture_packets = runtime_snapshot_value
                .as_ref()
                .map(runtime_packet_observations_from_snapshot)
                .transpose()?
                .unwrap_or_default();
            packet_observations_data.extend(runtime_capture_packets);
            if let Some(snapshot) = runtime_snapshot_value.as_ref() {
                for line in runtime_packet_observation_lines(snapshot)? {
                    packet_observations_data.push(line);
                }
            }
            let runtime_peer_observations = runtime_snapshot_value
                .as_ref()
                .map(runtime_peer_observations_from_snapshot)
                .unwrap_or_default();
            let route_source = load_route_source(route_output.as_deref(), route_scan);
            let route_observations = if route_source.error.is_none() {
                lai_core::parse_windows_ipv4_routes(&route_source.raw_output)
            } else {
                Vec::new()
            };
            let route_count = route_observations.len();
            let ping_source = load_ping_source(ping_output.as_deref(), ping_test.as_deref());
            let mut tunnel = if let Some(source) = ping_source.as_ref() {
                if source.error.is_none() && !source.raw_output.trim().is_empty() {
                    parse_windows_ping_observation(&source.raw_output, expected_peers)
                } else {
                    TunnelObservation {
                        state: tunnel_state,
                        connected_peer_count: connected_peers,
                        latency_ms,
                        packet_loss_percent,
                        path: None,
                    }
                }
            } else if let Some(runtime_tunnel) = runtime_tunnel {
                runtime_tunnel
            } else {
                TunnelObservation {
                    state: tunnel_state,
                    connected_peer_count: connected_peers,
                    latency_ms,
                    packet_loss_percent,
                    path: None,
                }
            };
            if connection_path.is_some() {
                tunnel.path = connection_path;
            }
            let expected_peers = if expected_peers == 0 && !runtime_peer_observations.is_empty() {
                runtime_peer_observations.len().min(u16::MAX as usize) as u16
            } else {
                expected_peers
            };
            let report = evaluate_network_observations(NetworkObservationSnapshot {
                adapter,
                tunnel: Some(tunnel),
                packets: packet_observations_data,
                expected_peer_count: expected_peers,
                expected_broadcast_ports: parse_ports(&broadcast_ports)?,
                expected_game_ports: parse_ports(&game_ports)?,
                route_observations,
                runtime_peers: runtime_peer_observations,
            });
            let mut output = serde_json::to_value(&report)?;
            output["adapterSource"] = serde_json::to_value(adapter_source)?;
            output["pingSource"] = serde_json::to_value(ping_source)?;
            output["routeSource"] = serde_json::to_value(route_source)?;
            output["routeCount"] = serde_json::json!(route_count);
            output["runtimeSnapshotSource"] = serde_json::json!({
                "source": if runtime_snapshot.is_some() { "runtime-snapshot" } else { "not-provided" },
                "loaded": runtime_snapshot_value.is_some(),
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        Command::DiagnosticExport {
            out,
            adapter_name,
            expected_ip,
            assigned_ip,
            subnet,
            adapter_netsh_output,
            adapter_scan,
            firewall_netsh_output,
            firewall_scan,
            ping_test,
            ping_output,
            expected_peers,
            broadcast_ports,
            game_ports,
            packets,
            packet_observations,
            runtime_snapshot,
            route_output,
            route_scan,
            netstat_output,
            netstat_scan,
            game_name,
            catalog,
            steam_app_id,
            discovery,
            ports,
            compatibility,
            program,
            packet_io_backend,
            packet_io_probe,
            wintun_ring_capacity,
            wintun_probe_receive,
            wintun_receive_attempts,
            wintun_receive_poll_interval_ms,
            wintun_probe_send,
            relay_local_offer,
            relay_remote_offer,
            relay_p2p_status,
        } => {
            let expected_ip = parse_optional_ipv4(expected_ip.as_deref())?;
            let assigned_ip = parse_optional_ipv4(assigned_ip.as_deref())?;
            let subnet = parse_optional_subnet(subnet.as_deref())?;
            let broadcast_ports = parse_ports(&broadcast_ports)?;
            let game_ports = parse_ports(&game_ports)?;
            let packet_observations_path = packet_observations.clone();
            let mut packet_data =
                load_packet_observations(packet_observations.as_deref(), &packets);
            let (runtime_snapshot_value, runtime_snapshot_error) =
                load_runtime_snapshot(runtime_snapshot.as_deref());
            if let Some(snapshot) = runtime_snapshot_value.as_ref() {
                merge_runtime_packet_observations(&mut packet_data, snapshot);
            }
            let packet_io_plan =
                lai_core::create_virtual_packet_io_plan(&adapter_name, &packet_io_backend, 1420);
            let packet_io_plan_value = runtime_snapshot_value
                .as_ref()
                .and_then(|snapshot| snapshot.get("packetIoPlan").cloned())
                .or(Some(serde_json::to_value(packet_io_plan)?));
            let packet_io_probe_value = runtime_snapshot_value
                .as_ref()
                .and_then(|snapshot| snapshot.get("packetIoProbe").cloned())
                .or_else(|| {
                    packet_io_probe.then(|| {
                        runtime_packet_io_probe(
                            &packet_io_backend,
                            &RuntimePacketIoProbeOptions {
                                wintun_adapter_name: adapter_name.clone(),
                                wintun_ring_capacity,
                                wintun_probe_receive,
                                wintun_receive_attempts,
                                wintun_receive_poll_interval_ms,
                                wintun_probe_send,
                            },
                        )
                    })
                });
            let (relay_fallback_plan, connection_path_report, relay_fallback_error) =
                load_relay_fallback_for_export(
                    relay_local_offer.as_deref(),
                    relay_remote_offer.as_deref(),
                    &relay_p2p_status,
                );
            let game_profile = profile_from_catalog_or_args(
                catalog.as_deref(),
                game_name,
                steam_app_id.as_deref(),
                discovery,
                ports,
                compatibility,
            )?;
            let inputs = DiagnosticExportInputs {
                adapter_name: adapter_name.clone(),
                expected_ip,
                assigned_ip,
                subnet,
                expected_peers,
                ping_host: ping_test.clone(),
                packet_observations: packet_observations_path,
                broadcast_ports,
                game_ports,
                game_name: game_profile.game_name,
                discovery: game_profile.discovery,
                ports: game_profile.ports,
                compatibility: game_profile.compatibility,
                program,
            };
            let sources = DiagnosticExportSources {
                adapter_netsh: load_adapter_source(
                    &adapter_name,
                    adapter_netsh_output.as_deref(),
                    adapter_scan,
                ),
                firewall_netsh: load_firewall_source(
                    firewall_netsh_output.as_deref(),
                    firewall_scan,
                ),
                ping_output: load_ping_source(ping_output.as_deref(), ping_test.as_deref()),
                packets: packet_data.packets,
                packet_raw_lines: packet_data.raw_lines,
                packet_error: packet_data.error,
                packet_io_plan: packet_io_plan_value,
                packet_io_probe: packet_io_probe_value,
                runtime_snapshot: runtime_snapshot_value,
                runtime_snapshot_error,
                route_table: load_route_source(route_output.as_deref(), route_scan),
                netstat_table: load_netstat_source(netstat_output.as_deref(), netstat_scan),
                relay_fallback_plan,
                connection_path_report,
                relay_fallback_error,
            };
            let bundle = create_diagnostic_export_bundle(
                current_epoch_ms(),
                diagnostic_environment()?,
                inputs,
                sources,
            );
            write_json_file(&out, &bundle)?;
            let bytes_written = fs::metadata(&out)?.len();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "ok",
                    "path": fs::canonicalize(&out)
                        .unwrap_or_else(|_| Path::new(&out).to_path_buf())
                        .display()
                        .to_string(),
                    "bytesWritten": bytes_written,
                    "bundleStatus": bundle.status,
                }))?
            );
        }
        Command::WintunDetect => {
            let report = lai_core::detect_wintun_availability();
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::WintunAdapterCreate {
            adapter_name,
            tunnel_type,
            yes,
        } => {
            if !yes {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "needs-confirmation",
                        "adapterName": adapter_name,
                        "tunnelType": tunnel_type,
                        "requiresElevation": true,
                        "confirmed": false,
                        "canExecuteNow": false,
                        "nextAction": "Review the request, then rerun with --yes true from an Administrator terminal.",
                    }))?
                );
                return Ok(());
            }
            let request = lai_core::WintunAdapterCreateRequest {
                adapter_name,
                tunnel_type,
            };
            let report = lai_core::create_wintun_adapter(request);
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::WintunAdapterEnsure {
            adapter_name,
            tunnel_type,
            yes,
        } => {
            if !yes {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "needs-confirmation",
                        "adapterName": adapter_name,
                        "tunnelType": tunnel_type,
                        "requiresElevation": true,
                        "confirmed": false,
                        "canExecuteNow": false,
                        "nextAction": "Review the request, then rerun with --yes true from an Administrator terminal.",
                    }))?
                );
                return Ok(());
            }
            let create_report =
                lai_core::create_wintun_adapter(lai_core::WintunAdapterCreateRequest {
                    adapter_name: adapter_name.clone(),
                    tunnel_type: tunnel_type.clone(),
                });
            let open_report = lai_core::open_wintun_adapter(lai_core::WintunAdapterOpenRequest {
                adapter_name: adapter_name.clone(),
            });
            let create_ok = matches!(
                create_report.status.as_str(),
                "created" | "adapter-exists" | "already-exists"
            );
            let open_ok = open_report.opened;
            let status = if open_ok {
                "ready"
            } else if create_ok {
                "created-not-opened"
            } else {
                "unavailable"
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": status,
                    "adapterName": adapter_name,
                    "tunnelType": tunnel_type,
                    "requiresElevation": true,
                    "confirmed": true,
                    "createReport": create_report,
                    "openReport": open_report,
                    "nextAction": if open_ok {
                        "Continue with adapter IP configuration and firewall rules."
                    } else {
                        "Check wintun.dll, Administrator permission, and Windows driver installation."
                    },
                }))?
            );
        }
        Command::WintunAdapterDelete {
            adapter_name,
            tunnel_type,
            force_close_sessions,
            yes,
        } => {
            if !yes {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "needs-confirmation",
                        "adapterName": adapter_name,
                        "tunnelType": tunnel_type,
                        "forceCloseSessions": force_close_sessions,
                        "requiresElevation": true,
                        "confirmed": false,
                        "canExecuteNow": false,
                        "nextAction": "Review the request, then rerun with --yes true from an Administrator terminal.",
                    }))?
                );
                return Ok(());
            }
            let request = lai_core::WintunAdapterDeleteRequest {
                adapter_name,
                tunnel_type,
                force_close_sessions,
            };
            let report = lai_core::delete_wintun_adapter(request);
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::WintunAdapterOpen {
            adapter_name,
            tunnel_type: _,
        } => {
            let request = lai_core::WintunAdapterOpenRequest { adapter_name };
            let report = lai_core::open_wintun_adapter(request);
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::WintunSessionProbe {
            adapter_name,
            tunnel_type,
            ring_capacity,
        } => {
            let request = lai_core::WintunSessionProbeRequest {
                adapter_name,
                tunnel_type,
                ring_capacity,
            };
            let report = lai_core::probe_wintun_session(request);
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::WintunPacketSendProbe {
            adapter_name,
            ring_capacity,
            source_ip,
            destination_ip,
            source_port,
            destination_port,
            message,
            broadcast,
            yes,
        } => {
            if !yes {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "needs-confirmation",
                        "adapterName": adapter_name,
                        "ringCapacity": ring_capacity,
                        "packet": {
                            "sourceIp": source_ip,
                            "destinationIp": destination_ip,
                            "sourcePort": source_port,
                            "destinationPort": destination_port,
                            "payloadBytes": message.as_bytes().len(),
                            "broadcast": broadcast,
                        },
                        "requiresElevation": true,
                        "confirmed": false,
                        "canExecuteNow": false,
                        "nextAction": "Review the packet probe, then rerun with --yes true from an Administrator terminal with wintun.dll available.",
                    }))?
                );
                return Ok(());
            }
            let packet = VirtualUdpPacket {
                source_ip: source_ip.parse::<Ipv4Addr>()?,
                destination_ip: destination_ip.parse::<Ipv4Addr>()?,
                source_port,
                destination_port,
                payload: message.into_bytes(),
                broadcast,
            };
            let request = lai_core::WintunPacketSendProbeRequest {
                adapter_name,
                ring_capacity,
                packet,
            };
            let report = lai_core::probe_wintun_packet_send(request);
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::WintunPacketReceiveProbe {
            adapter_name,
            ring_capacity,
            max_attempts,
            poll_interval_ms,
        } => {
            let request = lai_core::WintunPacketReceiveProbeRequest {
                adapter_name,
                ring_capacity,
                max_attempts,
                poll_interval_ms,
            };
            let report = lai_core::probe_wintun_packet_receive(request);
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    Ok(())
}

fn profile_from_args(
    game_name: String,
    discovery: String,
    ports: String,
    compatibility: String,
) -> Result<GameProfile, Box<dyn std::error::Error>> {
    Ok(GameProfile {
        game_name,
        steam_app_id: None,
        discovery: parse_discovery(&discovery)?,
        ports: parse_ports(&ports)?,
        join_method: "lan_list_or_direct_ip".to_owned(),
        compatibility: parse_compatibility(&compatibility)?,
        notes: String::new(),
    })
}

fn profile_from_catalog_or_args(
    catalog: Option<&str>,
    game_name: String,
    steam_app_id: Option<&str>,
    discovery: String,
    ports: String,
    compatibility: String,
) -> Result<GameProfile, Box<dyn std::error::Error>> {
    if let Some(catalog_path) = catalog.map(str::trim).filter(|path| !path.is_empty()) {
        let catalog_text = fs::read_to_string(catalog_path)?;
        let catalog = parse_game_profile_catalog_json(&catalog_text)?;
        if let Some(matched) = find_game_profile(&catalog, Some(&game_name), steam_app_id) {
            return Ok(matched.profile);
        }
    }
    profile_from_args(game_name, discovery, ports, compatibility)
}

fn parse_ports(value: &str) -> Result<Vec<u16>, Box<dyn std::error::Error>> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(',')
        .filter(|item| !item.trim().is_empty())
        .map(|item| {
            item.trim()
                .parse::<u16>()
                .map_err(|err| invalid_input(format!("invalid port `{item}`: {err}")))
        })
        .collect()
}

fn parse_runtime_peers(
    values: &[String],
) -> Result<Vec<RoomRuntimePeer>, Box<dyn std::error::Error>> {
    values
        .iter()
        .map(|value| {
            let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
            if parts.len() != 3 {
                return Err(invalid_input(format!(
                    "invalid peer `{value}`, expected peer_id,virtual_ip,endpoint"
                )));
            }
            Ok(RoomRuntimePeer {
                peer_id: parts[0].to_owned(),
                virtual_ip: parts[1].parse::<Ipv4Addr>().map_err(|err| {
                    invalid_input(format!("invalid peer virtual IP `{}`: {err}", parts[1]))
                })?,
                endpoint: parts[2].to_owned(),
            })
        })
        .collect()
}

fn parse_runtime_peers_with_bootstrap(
    values: &[String],
    bootstrap_values: &[String],
) -> Result<Vec<RoomRuntimePeer>, Box<dyn std::error::Error>> {
    let mut peers = parse_runtime_peers(values)?;
    for value in bootstrap_values {
        peers.push(parse_runtime_peer_from_bootstrap(value)?);
    }
    Ok(peers)
}

fn parse_runtime_peer_from_bootstrap(
    value: &str,
) -> Result<RoomRuntimePeer, Box<dyn std::error::Error>> {
    let parts = value.splitn(3, ',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(invalid_input(format!(
            "invalid NAT bootstrap peer `{value}`, expected peer_id,virtual_ip,result-json-or-file"
        )));
    }
    let peer_id = parts[0].to_owned();
    let virtual_ip = parts[1].parse::<Ipv4Addr>().map_err(|err| {
        invalid_input(format!(
            "invalid NAT bootstrap peer virtual IP `{}`: {err}",
            parts[1]
        ))
    })?;
    let result = load_json_argument(parts[2])?;
    runtime_peer_from_bootstrap_result(&peer_id, virtual_ip, &result)
}

fn runtime_peer_from_bootstrap_result(
    peer_id: &str,
    virtual_ip: Ipv4Addr,
    result: &serde_json::Value,
) -> Result<RoomRuntimePeer, Box<dyn std::error::Error>> {
    let endpoint = match runtime_direct_endpoint_from_bootstrap_result(peer_id, result)? {
        Some(endpoint) => endpoint,
        None => runtime_fallback_endpoint_from_bootstrap_result(result)?,
    };

    Ok(RoomRuntimePeer {
        peer_id: peer_id.to_owned(),
        virtual_ip,
        endpoint,
    })
}

fn runtime_direct_endpoint_from_bootstrap_result(
    peer_id: &str,
    result: &serde_json::Value,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let Some(selected) = result.get("selectedPeer") else {
        return Ok(None);
    };
    let responder_peer_id = selected
        .get("responderPeerId")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            invalid_input("NAT bootstrap selectedPeer is missing responderPeerId".to_owned())
        })?;
    if responder_peer_id != peer_id {
        return Err(invalid_input(format!(
            "NAT bootstrap responder `{responder_peer_id}` does not match expected peer `{peer_id}`"
        )));
    }
    let accepted = selected
        .get("accepted")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let nonce_matched = selected
        .get("nonceMatched")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !accepted || !nonce_matched {
        return Ok(None);
    }
    selected
        .get("endpoint")
        .and_then(serde_json::Value::as_str)
        .map(|endpoint| Ok(endpoint.to_owned()))
        .unwrap_or_else(|| {
            Err(invalid_input(
                "NAT bootstrap selectedPeer is missing endpoint".to_owned(),
            ))
        })
        .map(Some)
}

fn runtime_fallback_endpoint_from_bootstrap_result(
    result: &serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let local_offer: lai_core::NatTraversalOffer =
        serde_json::from_value(result.get("localOffer").cloned().ok_or_else(|| {
            invalid_input("NAT bootstrap result is missing localOffer".to_owned())
        })?)?;
    let remote_offer: lai_core::NatTraversalOffer =
        serde_json::from_value(result.get("remoteOffer").cloned().ok_or_else(|| {
            invalid_input("NAT bootstrap result is missing remoteOffer".to_owned())
        })?)?;
    let status = result
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let report = lai_core::evaluate_connection_path(
        &local_offer,
        &remote_offer,
        connection_path_status_from_bootstrap_status(status),
    );
    report
        .selected_endpoints
        .first()
        .cloned()
        .ok_or_else(|| {
            invalid_input(
                "NAT bootstrap did not produce a direct peer and no relay/P2P fallback endpoint was available."
                    .to_owned(),
            )
        })
}

fn run_runtime_nat_bootstraps(
    values: &[String],
    room_id: &str,
    local_peer_id: &str,
    local_virtual_ip: Ipv4Addr,
    key: &str,
    bind: &str,
    attempts: u16,
    interval_ms: u64,
    timeout_ms: u64,
    stun_server: Option<&str>,
    stun_timeout_ms: u64,
    upnp_port_map: bool,
    upnp_timeout_ms: u64,
    upnp_lease_seconds: u32,
    upnp_gateway_location: Option<&str>,
) -> Result<(Vec<RoomRuntimePeer>, Vec<serde_json::Value>), Box<dyn std::error::Error>> {
    let mut peers = Vec::new();
    let mut results = Vec::new();
    for value in values {
        let parts = value.splitn(3, ',').map(str::trim).collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(invalid_input(format!(
                "invalid NAT bootstrap remote peer `{value}`, expected peer_id,virtual_ip,remote-offer-json-or-file"
            )));
        }
        let remote_peer_id = parts[0].to_owned();
        let remote_virtual_ip = parts[1].parse::<Ipv4Addr>().map_err(|err| {
            invalid_input(format!(
                "invalid NAT bootstrap remote peer virtual IP `{}`: {err}",
                parts[1]
            ))
        })?;
        let remote_offer = load_nat_offer_argument(parts[2])?;
        let result = run_nat_p2p_bootstrap(
            room_id,
            local_peer_id,
            local_virtual_ip,
            key,
            bind,
            &remote_offer,
            None,
            stun_server,
            stun_timeout_ms,
            upnp_port_map,
            upnp_timeout_ms,
            upnp_lease_seconds,
            upnp_gateway_location,
            Vec::new(),
            attempts,
            interval_ms,
            timeout_ms,
        )?;
        peers.push(runtime_peer_from_bootstrap_result(
            &remote_peer_id,
            remote_virtual_ip,
            &result,
        )?);
        results.push(result);
    }
    Ok((peers, results))
}

fn run_runtime_coordination_bootstraps(
    store_path: Option<&str>,
    peer_specs: &[String],
    room_id: &str,
    local_peer_id: &str,
    local_virtual_ip: Ipv4Addr,
    key: &str,
    bind: &str,
    attempts: u16,
    interval_ms: u64,
    timeout_ms: u64,
    stun_server: Option<&str>,
    stun_timeout_ms: u64,
    upnp_port_map: bool,
    upnp_timeout_ms: u64,
    upnp_lease_seconds: u32,
    upnp_gateway_location: Option<&str>,
) -> Result<(Vec<RoomRuntimePeer>, Vec<serde_json::Value>), Box<dyn std::error::Error>> {
    let Some(store_path) = store_path else {
        return Ok((Vec::new(), Vec::new()));
    };
    let peer_specs = parse_coordination_peer_specs(peer_specs)?;
    if peer_specs.is_empty() {
        return Err(invalid_input(
            "--coordination-store requires at least one --coordination-peer peer_id,virtual_ip"
                .to_owned(),
        ));
    }
    let mut store = load_coordination_store_or_default(store_path)?;
    let fetch = lai_core::fetch_coordination_offers(
        &mut store,
        room_id.to_owned(),
        local_peer_id.to_owned(),
        current_epoch_ms(),
    );
    write_json_file(store_path, &store)?;
    let mut peers = Vec::new();
    let mut bootstrap_results = Vec::new();
    let mut missing_peers = Vec::new();

    for (remote_peer_id, remote_virtual_ip) in peer_specs {
        let Some(offer) = fetch
            .offers
            .iter()
            .find(|offer| offer.peer_id == remote_peer_id)
        else {
            missing_peers.push(remote_peer_id);
            continue;
        };
        let result = run_nat_p2p_bootstrap(
            room_id,
            local_peer_id,
            local_virtual_ip,
            key,
            bind,
            offer,
            None,
            stun_server,
            stun_timeout_ms,
            upnp_port_map,
            upnp_timeout_ms,
            upnp_lease_seconds,
            upnp_gateway_location,
            Vec::new(),
            attempts,
            interval_ms,
            timeout_ms,
        )?;
        peers.push(runtime_peer_from_bootstrap_result(
            &remote_peer_id,
            remote_virtual_ip,
            &result,
        )?);
        bootstrap_results.push(serde_json::json!({
            "source": "coordination-store",
            "store": store_path,
            "peerId": remote_peer_id,
            "result": result,
        }));
    }

    if !missing_peers.is_empty() {
        return Err(invalid_input(format!(
            "coordination store did not contain active offers for peer(s): {}",
            missing_peers.join(",")
        )));
    }
    if bootstrap_results.is_empty() {
        bootstrap_results.push(serde_json::json!({
            "source": "coordination-store",
            "store": store_path,
            "fetch": fetch,
            "status": "empty",
        }));
    } else {
        bootstrap_results.insert(
            0,
            serde_json::json!({
                "source": "coordination-store",
                "store": store_path,
                "fetch": fetch,
                "status": "ok",
            }),
        );
    }

    Ok((peers, bootstrap_results))
}

fn run_runtime_coordination_server_bootstraps(
    server: Option<&str>,
    peer_specs: &[String],
    room_id: &str,
    local_peer_id: &str,
    local_virtual_ip: Ipv4Addr,
    key: &str,
    bind: &str,
    attempts: u16,
    interval_ms: u64,
    timeout_ms: u64,
    stun_server: Option<&str>,
    stun_timeout_ms: u64,
    upnp_port_map: bool,
    upnp_timeout_ms: u64,
    upnp_lease_seconds: u32,
    upnp_gateway_location: Option<&str>,
) -> Result<(Vec<RoomRuntimePeer>, Vec<serde_json::Value>), Box<dyn std::error::Error>> {
    let Some(server) = server else {
        return Ok((Vec::new(), Vec::new()));
    };
    let peer_specs = parse_coordination_peer_specs(peer_specs)?;
    if peer_specs.is_empty() {
        return Err(invalid_input(
            "--coordination-server requires at least one --coordination-peer peer_id,virtual_ip"
                .to_owned(),
        ));
    }
    let fetch_value = coordination_http_fetch_offers(server, room_id, local_peer_id)?;
    let fetch: lai_core::CoordinationFetchResult = serde_json::from_value(fetch_value.clone())?;
    let mut peers = Vec::new();
    let mut bootstrap_results = Vec::new();
    let mut missing_peers = Vec::new();

    for (remote_peer_id, remote_virtual_ip) in peer_specs {
        let Some(offer) = fetch
            .offers
            .iter()
            .find(|offer| offer.peer_id == remote_peer_id)
        else {
            missing_peers.push(remote_peer_id);
            continue;
        };
        let result = run_nat_p2p_bootstrap(
            room_id,
            local_peer_id,
            local_virtual_ip,
            key,
            bind,
            offer,
            None,
            stun_server,
            stun_timeout_ms,
            upnp_port_map,
            upnp_timeout_ms,
            upnp_lease_seconds,
            upnp_gateway_location,
            Vec::new(),
            attempts,
            interval_ms,
            timeout_ms,
        )?;
        peers.push(runtime_peer_from_bootstrap_result(
            &remote_peer_id,
            remote_virtual_ip,
            &result,
        )?);
        bootstrap_results.push(serde_json::json!({
            "source": "coordination-http",
            "server": server,
            "peerId": remote_peer_id,
            "result": result,
        }));
    }

    if !missing_peers.is_empty() {
        return Err(invalid_input(format!(
            "coordination server did not contain active offers for peer(s): {}",
            missing_peers.join(",")
        )));
    }
    if bootstrap_results.is_empty() {
        bootstrap_results.push(serde_json::json!({
            "source": "coordination-http",
            "server": server,
            "fetch": fetch_value,
            "status": "empty",
        }));
    } else {
        bootstrap_results.insert(
            0,
            serde_json::json!({
                "source": "coordination-http",
                "server": server,
                "fetch": fetch_value,
                "status": "ok",
            }),
        );
    }

    Ok((peers, bootstrap_results))
}

fn parse_coordination_peer_specs(
    values: &[String],
) -> Result<Vec<(String, Ipv4Addr)>, Box<dyn std::error::Error>> {
    values
        .iter()
        .map(|value| {
            let parts = value.splitn(2, ',').map(str::trim).collect::<Vec<_>>();
            if parts.len() != 2 {
                return Err(invalid_input(format!(
                    "invalid coordination peer `{value}`, expected peer_id,virtual_ip"
                )));
            }
            let virtual_ip = parts[1].parse::<Ipv4Addr>().map_err(|err| {
                invalid_input(format!(
                    "invalid coordination peer virtual IP `{}`: {err}",
                    parts[1]
                ))
            })?;
            Ok((parts[0].to_owned(), virtual_ip))
        })
        .collect()
}

fn load_json_argument(value: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let text = if Path::new(value).exists() {
        let bytes = fs::read(value)?;
        let bytes = bytes
            .strip_prefix(&[0xef, 0xbb, 0xbf])
            .unwrap_or(bytes.as_slice());
        String::from_utf8(bytes.to_vec())?
    } else {
        value.to_owned()
    };
    Ok(serde_json::from_str(text.trim_start_matches('\u{feff}'))?)
}

fn parse_discovery(value: &str) -> Result<DiscoveryMode, Box<dyn std::error::Error>> {
    match value {
        "udp_broadcast" => Ok(DiscoveryMode::UdpBroadcast),
        "direct_ip" => Ok(DiscoveryMode::DirectIp),
        "manual_ports" => Ok(DiscoveryMode::ManualPorts),
        "unknown" => Ok(DiscoveryMode::Unknown),
        other => Err(invalid_input(format!(
            "unsupported discovery mode `{other}`"
        ))),
    }
}

fn parse_compatibility(value: &str) -> Result<CompatibilityLevel, Box<dyn std::error::Error>> {
    match value {
        "A" | "a" => Ok(CompatibilityLevel::A),
        "B" | "b" => Ok(CompatibilityLevel::B),
        "C" | "c" => Ok(CompatibilityLevel::C),
        "D" | "d" => Ok(CompatibilityLevel::D),
        "unknown" => Ok(CompatibilityLevel::Unknown),
        other => Err(invalid_input(format!(
            "unsupported compatibility level `{other}`"
        ))),
    }
}

fn parse_optional_ipv4(
    value: Option<&str>,
) -> Result<Option<Ipv4Addr>, Box<dyn std::error::Error>> {
    value
        .map(|item| {
            item.parse::<Ipv4Addr>()
                .map_err(|err| invalid_input(format!("invalid IPv4 address `{item}`: {err}")))
        })
        .transpose()
}

fn parse_optional_subnet(
    value: Option<&str>,
) -> Result<Option<Ipv4Subnet>, Box<dyn std::error::Error>> {
    value
        .map(|item| {
            item.parse::<Ipv4Subnet>()
                .map_err(|err| invalid_input(format!("invalid CIDR `{item}`: {err}")))
        })
        .transpose()
}

fn parse_packet_observations(
    value: &str,
) -> Result<Vec<PacketObservation>, Box<dyn std::error::Error>> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(value
        .split(',')
        .filter(|item| !item.trim().is_empty())
        .map(lai_core::parse_packet_observation_line)
        .collect::<lai_core::Result<Vec<_>>>()?)
}

fn observed_firewall_rules(
    expected_rules: &[FirewallRule],
    observed: &str,
    program: Option<String>,
) -> Result<Vec<FirewallRuleObservation>, Box<dyn std::error::Error>> {
    if observed.trim().is_empty() {
        return Ok(Vec::new());
    }
    let observed_ports = observed
        .split(',')
        .filter(|item| !item.trim().is_empty())
        .map(parse_observed_port)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(expected_rules
        .iter()
        .filter(|rule| {
            observed_ports.iter().any(|(protocol, port)| {
                rule.protocol.eq_ignore_ascii_case(protocol) && rule.port == *port
            })
        })
        .map(|rule| observation_from_expected_rule(rule, program.clone()))
        .collect())
}

fn parse_observed_port(value: &str) -> Result<(String, u16), Box<dyn std::error::Error>> {
    let (protocol, port) = value.trim().split_once(':').ok_or_else(|| {
        invalid_input(format!(
            "invalid observed rule `{value}`, expected protocol:port"
        ))
    })?;
    let protocol = protocol.trim().to_lowercase();
    if protocol != "udp" && protocol != "tcp" {
        return Err(invalid_input(format!(
            "unsupported observed protocol `{protocol}`"
        )));
    }
    let port = port
        .trim()
        .parse::<u16>()
        .map_err(|err| invalid_input(format!("invalid observed port `{port}`: {err}")))?;
    Ok((protocol, port))
}

struct PacketLoadResult {
    packets: Vec<PacketObservation>,
    raw_lines: Vec<String>,
    error: Option<String>,
}

fn load_adapter_source(
    adapter_name: &str,
    adapter_netsh_output: Option<&str>,
    adapter_scan: bool,
) -> DiagnosticTextSource {
    if let Some(path) = adapter_netsh_output {
        return read_text_source("netsh-file", path);
    }
    if adapter_scan {
        return run_text_source(
            "netsh-scan",
            "netsh",
            &[
                "interface",
                "ipv4",
                "show",
                "config",
                &format!("name={adapter_name}"),
            ],
        );
    }
    DiagnosticTextSource {
        source: "manual-input".to_owned(),
        raw_output: String::new(),
        error: None,
    }
}

fn load_firewall_source(
    firewall_netsh_output: Option<&str>,
    firewall_scan: bool,
) -> DiagnosticTextSource {
    if let Some(path) = firewall_netsh_output {
        return read_text_source("netsh-file", path);
    }
    if firewall_scan {
        return run_text_source(
            "netsh-scan",
            "netsh",
            &["advfirewall", "firewall", "show", "rule", "name=all"],
        );
    }
    DiagnosticTextSource {
        source: "skipped".to_owned(),
        raw_output: String::new(),
        error: None,
    }
}

fn load_route_source(route_output: Option<&str>, route_scan: bool) -> DiagnosticTextSource {
    if let Some(path) = route_output {
        return read_text_source("route-file", path);
    }
    if route_scan {
        return run_text_source("route-scan", "route", &["print", "-4"]);
    }
    DiagnosticTextSource {
        source: "skipped".to_owned(),
        raw_output: String::new(),
        error: None,
    }
}

fn load_netstat_source(netstat_output: Option<&str>, netstat_scan: bool) -> DiagnosticTextSource {
    if let Some(path) = netstat_output {
        return read_text_source("netstat-file", path);
    }
    if netstat_scan {
        return run_text_source("netstat-scan", "netstat", &["-ano"]);
    }
    DiagnosticTextSource {
        source: "skipped".to_owned(),
        raw_output: String::new(),
        error: None,
    }
}

fn load_ping_source(
    ping_output: Option<&str>,
    ping_test: Option<&str>,
) -> Option<DiagnosticTextSource> {
    if let Some(path) = ping_output {
        return Some(read_text_source("ping-file", path));
    }
    ping_test.map(|host| run_text_source("ping-test", "ping", &["-n", "4", host]))
}

fn read_text_source(source: &str, path: &str) -> DiagnosticTextSource {
    match fs::read_to_string(path) {
        Ok(raw_output) => DiagnosticTextSource {
            source: source.to_owned(),
            raw_output,
            error: None,
        },
        Err(err) => DiagnosticTextSource {
            source: source.to_owned(),
            raw_output: String::new(),
            error: Some(err.to_string()),
        },
    }
}

fn run_text_source(source: &str, program: &str, args: &[&str]) -> DiagnosticTextSource {
    match ProcessCommand::new(program).args(args).output() {
        Ok(output) => {
            let mut raw_output = String::from_utf8_lossy(&output.stdout).to_string();
            if !output.stderr.is_empty() {
                raw_output.push_str(&String::from_utf8_lossy(&output.stderr));
            }
            DiagnosticTextSource {
                source: source.to_owned(),
                raw_output,
                error: if output.status.success() {
                    None
                } else {
                    Some(format!("{program} exited with status {}", output.status))
                },
            }
        }
        Err(err) => DiagnosticTextSource {
            source: source.to_owned(),
            raw_output: String::new(),
            error: Some(err.to_string()),
        },
    }
}

fn load_packet_observations(packet_observations: Option<&str>, packets: &str) -> PacketLoadResult {
    let mut raw_lines = Vec::new();
    let mut error = None;

    if let Some(path) = packet_observations {
        match fs::read_to_string(path) {
            Ok(text) => raw_lines.extend(
                text.lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty() && !line.starts_with('#'))
                    .map(str::to_owned),
            ),
            Err(err) => error = Some(err.to_string()),
        }
    }
    raw_lines.extend(
        packets
            .split(',')
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned),
    );

    let mut parsed = Vec::new();
    if error.is_none() {
        for line in &raw_lines {
            match lai_core::parse_packet_observation_line(line) {
                Ok(packet) => parsed.push(packet),
                Err(err) => {
                    error = Some(err.to_string());
                    parsed.clear();
                    break;
                }
            }
        }
    }

    PacketLoadResult {
        packets: parsed,
        raw_lines,
        error,
    }
}

fn load_runtime_snapshot(path: Option<&str>) -> (Option<serde_json::Value>, Option<String>) {
    let Some(path) = path else {
        return (None, None);
    };
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(value) => (Some(value), None),
            Err(err) => (None, Some(format!("runtime snapshot parse failed: {err}"))),
        },
        Err(err) => (None, Some(format!("runtime snapshot read failed: {err}"))),
    }
}

fn load_runtime_cleanup_plan_for_report(
    cleanup_plan: Option<&str>,
    runtime_snapshot: Option<&serde_json::Value>,
) -> Result<lai_core::RuntimeCleanupPlan, Box<dyn std::error::Error>> {
    if let Some(plan) = cleanup_plan {
        return serde_json::from_value(load_json_argument(plan)?)
            .map_err(|err| invalid_input(format!("invalid runtime cleanup plan: {err}")).into());
    }
    let Some(plan_value) =
        runtime_snapshot.and_then(|snapshot| snapshot.get("runtimeCleanupPlan").cloned())
    else {
        return Err(invalid_input(
            "runtime-cleanup-report requires --cleanup-plan or a --runtime-snapshot containing runtimeCleanupPlan".to_owned(),
        ));
    };
    serde_json::from_value(plan_value).map_err(|err| {
        invalid_input(format!("invalid runtime cleanup plan in snapshot: {err}")).into()
    })
}

fn runtime_wintun_close_report_from_snapshot(
    runtime_snapshot: &serde_json::Value,
) -> Option<lai_core::WintunPacketIoCloseReport> {
    runtime_snapshot
        .get("wintunRuntime")
        .and_then(|runtime| runtime.get("close"))
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
}

fn merge_runtime_packet_observations(
    packet_data: &mut PacketLoadResult,
    runtime_snapshot: &serde_json::Value,
) {
    let Some(lines) = runtime_snapshot
        .get("packetObservationLines")
        .and_then(serde_json::Value::as_array)
    else {
        return;
    };
    for line in lines.iter().filter_map(serde_json::Value::as_str) {
        let line = line.trim();
        if !line.is_empty() {
            packet_data.raw_lines.push(line.to_owned());
        }
    }
    if packet_data.error.is_none() {
        packet_data.packets.clear();
        for line in &packet_data.raw_lines {
            match lai_core::parse_packet_observation_line(line) {
                Ok(packet) => packet_data.packets.push(packet),
                Err(err) => {
                    packet_data.error = Some(err.to_string());
                    packet_data.packets.clear();
                    break;
                }
            }
        }
    }
}

fn runtime_packet_observations_from_snapshot(
    runtime_snapshot: &serde_json::Value,
) -> Result<Vec<PacketObservation>, Box<dyn std::error::Error>> {
    let Some(captures) = runtime_snapshot
        .get("packetCaptureSummaries")
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(Vec::new());
    };
    captures
        .iter()
        .cloned()
        .map(serde_json::from_value::<PacketCaptureSummary>)
        .map(|result| {
            result
                .map(|summary| lai_core::packet_observation_from_capture_summary(&summary))
                .map_err(|err| err.into())
        })
        .collect()
}

fn runtime_packet_observation_lines(
    runtime_snapshot: &serde_json::Value,
) -> Result<Vec<PacketObservation>, Box<dyn std::error::Error>> {
    let Some(lines) = runtime_snapshot
        .get("packetObservationLines")
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(Vec::new());
    };
    lines
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(lai_core::parse_packet_observation_line)
        .map(|result| result.map_err(|err| err.into()))
        .collect()
}

fn runtime_peer_observations_from_snapshot(
    runtime_snapshot: &serde_json::Value,
) -> Vec<lai_core::RuntimePeerObservation> {
    let summaries = runtime_snapshot
        .get("runtimePeerSummaries")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    runtime_peer_observations_from_summaries(&summaries)
}

fn diagnostic_environment() -> Result<DiagnosticExportEnvironment, Box<dyn std::error::Error>> {
    Ok(DiagnosticExportEnvironment {
        machine_name: std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_default(),
        user_name: std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_default(),
        os_version: std::env::consts::OS.to_owned(),
        current_directory: std::env::current_dir()?.display().to_string(),
        architecture: std::env::consts::ARCH.to_owned(),
    })
}

fn current_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn write_json_file<T: serde::Serialize>(
    path: &str,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    write_text_file_with_retry(
        path,
        &format!("{}\n", serde_json::to_string_pretty(value)?),
        12,
        Duration::from_millis(25),
    )?;
    Ok(())
}

fn read_text_file_with_retry(
    path: &str,
    attempts: usize,
    delay: Duration,
) -> Result<String, std::io::Error> {
    let mut last_error = None;
    for attempt in 0..attempts.max(1) {
        match fs::read_to_string(path) {
            Ok(text) => return Ok(text),
            Err(err) => {
                if err.kind() == ErrorKind::NotFound || attempt + 1 >= attempts.max(1) {
                    return Err(err);
                }
                last_error = Some(err);
                std::thread::sleep(delay);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| std::io::Error::new(ErrorKind::Other, "failed to read file")))
}

fn write_text_file_with_retry(
    path: &str,
    text: &str,
    attempts: usize,
    delay: Duration,
) -> Result<(), std::io::Error> {
    let mut last_error = None;
    for attempt in 0..attempts.max(1) {
        match fs::write(path, text) {
            Ok(()) => return Ok(()),
            Err(err) => {
                if attempt + 1 >= attempts.max(1) {
                    return Err(err);
                }
                last_error = Some(err);
                std::thread::sleep(delay);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| std::io::Error::new(ErrorKind::Other, "failed to write file")))
}

fn detect_windows_elevation() -> Option<bool> {
    if !cfg!(windows) {
        return None;
    }
    ProcessCommand::new("net")
        .arg("session")
        .output()
        .ok()
        .map(|output| output.status.success())
}

fn runtime_cleanup_apply_status(
    preview: &lai_core::CommandExecutionPreview,
    command_results: &[CommandExecutionRecord],
    unsafe_commands: &[String],
) -> String {
    if !unsafe_commands.is_empty() {
        return "blocked-unsafe-command".to_owned();
    }
    if command_results.is_empty() {
        if preview.commands.is_empty() {
            "nothing-to-apply".to_owned()
        } else if !preview.confirmed {
            "needs-confirmation".to_owned()
        } else if preview.requires_elevation && !preview.can_execute_now {
            "needs-elevation".to_owned()
        } else {
            "planned".to_owned()
        }
    } else if command_results
        .iter()
        .all(|record| record.status == CommandExecutionStatus::Succeeded)
    {
        "applied".to_owned()
    } else {
        "failed".to_owned()
    }
}

fn runtime_cleanup_command_safety_errors(plan: &lai_core::RuntimeCleanupPlan) -> Vec<String> {
    let allowed = lai_core::create_windows_runtime_cleanup_plan_with_routes(
        plan.room_id.clone(),
        plan.local_peer_id.clone(),
        plan.local_virtual_ip,
        plan.virtual_subnet,
        plan.adapter_name.clone(),
        plan.packet_io_backend.clone(),
        plan.restore_adapter,
        plan.cleanup_routes,
    );
    plan.commands
        .iter()
        .filter(|command| {
            !allowed
                .commands
                .iter()
                .any(|allowed| command.tool == allowed.tool && command.args == allowed.args)
        })
        .map(|command| {
            format!(
                "Rejected cleanup command not generated from the current plan fields: {}",
                command.command
            )
        })
        .collect()
}

fn route_matches_room(
    route: &lai_core::WindowsRouteObservation,
    virtual_ip: Option<Ipv4Addr>,
    subnet: Option<Ipv4Subnet>,
) -> bool {
    if route.destination.prefix == 0 {
        return false;
    }
    virtual_ip.is_some_and(|ip| route.destination.contains(ip))
        || subnet.is_some_and(|subnet| route.destination.intersects(subnet))
}

fn parse_protocol_filter(value: &str) -> Vec<String> {
    let mut protocols = value
        .split(',')
        .map(str::trim)
        .filter(|protocol| !protocol.is_empty())
        .map(str::to_ascii_lowercase)
        .filter(|protocol| protocol == "tcp" || protocol == "udp")
        .collect::<Vec<_>>();
    protocols.sort();
    protocols.dedup();
    protocols
}

fn load_or_create_game_plan(
    game_plan: Option<&str>,
    catalog: Option<&str>,
    game_name: String,
    steam_app_id: Option<&str>,
    subnet: String,
    discovery: String,
    ports: String,
    compatibility: String,
    host_ip: Option<&str>,
    local_ip: Option<&str>,
) -> Result<lai_core::GameNetworkPlan, Box<dyn std::error::Error>> {
    if let Some(plan) = game_plan {
        return serde_json::from_value(load_json_argument(plan)?)
            .map_err(|err| invalid_input(format!("invalid game network plan: {err}")).into());
    }
    let profile = profile_from_catalog_or_args(
        catalog,
        game_name,
        steam_app_id,
        discovery,
        ports,
        compatibility,
    )?;
    Ok(create_game_network_plan(
        &profile,
        subnet.parse::<Ipv4Subnet>()?,
        parse_optional_ipv4(host_ip)?,
        parse_optional_ipv4(local_ip)?,
        30,
    ))
}

fn game_plan_ports(plan: &lai_core::GameNetworkPlan) -> Vec<u16> {
    let mut ports = plan
        .firewall_rules
        .iter()
        .map(|rule| rule.port)
        .collect::<Vec<_>>();
    ports.sort_unstable();
    ports.dedup();
    ports
}

fn game_readiness_firewall_report(
    plan: &lai_core::GameNetworkPlan,
    source: &DiagnosticTextSource,
    program: Option<&str>,
    requested: bool,
) -> Option<FirewallDiagnosticsReport> {
    if !requested {
        return None;
    }
    if let Some(error) = source.error.as_ref() {
        return Some(FirewallDiagnosticsReport {
            status: "needs-attention".to_owned(),
            summary: format!("Firewall diagnostics failed: {error}"),
            expected_rule_count: plan.firewall_rules.len(),
            observed_rule_count: 0,
            problem_count: plan.firewall_rules.len().max(1),
            checks: Vec::new(),
        });
    }
    let observed_rules = parse_netsh_firewall_rules(&source.raw_output);
    Some(evaluate_firewall_diagnostics(
        &plan.firewall_rules,
        &observed_rules,
        program,
    ))
}

fn endpoint_matches_game_ports(
    endpoint: &lai_core::WindowsNetstatEndpoint,
    expected_ports: &[u16],
    expected_protocols: &[String],
) -> bool {
    endpoint
        .local_port
        .is_some_and(|port| expected_ports.contains(&port))
        && (expected_protocols.is_empty()
            || expected_protocols
                .iter()
                .any(|protocol| protocol.eq_ignore_ascii_case(&endpoint.protocol)))
}

fn execute_network_commands(commands: &[NetworkCommand]) -> Vec<CommandExecutionRecord> {
    commands.iter().map(execute_network_command).collect()
}

fn firewall_commands_as_network_commands(
    commands: &[lai_core::FirewallCommand],
) -> Vec<NetworkCommand> {
    commands
        .iter()
        .map(|command| NetworkCommand {
            tool: command.tool.clone(),
            args: command.args.clone(),
            command: command.command.clone(),
            purpose: command
                .purpose
                .clone()
                .unwrap_or_else(|| format!("Apply firewall rule {}", command.rule_name)),
        })
        .collect()
}

fn execute_firewall_commands(
    commands: &[lai_core::FirewallCommand],
) -> Vec<CommandExecutionRecord> {
    commands
        .iter()
        .map(|command| {
            execute_network_command(&NetworkCommand {
                tool: command.tool.clone(),
                args: command.args.clone(),
                command: command.command.clone(),
                purpose: command
                    .purpose
                    .clone()
                    .unwrap_or_else(|| format!("Apply firewall rule {}", command.rule_name)),
            })
        })
        .collect()
}

fn execute_network_command(command: &NetworkCommand) -> CommandExecutionRecord {
    match ProcessCommand::new(&command.tool)
        .args(&command.args)
        .output()
    {
        Ok(output) => {
            CommandExecutionRecord {
                command: command.command.clone(),
                purpose: command.purpose.clone(),
                status: if output.status.success() {
                    CommandExecutionStatus::Succeeded
                } else {
                    CommandExecutionStatus::Failed
                },
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                error: None,
                next_action: if output.status.success() {
                    None
                } else {
                    Some("Check that the adapter name exists and rerun from an Administrator terminal.".to_owned())
                },
            }
        }
        Err(err) => CommandExecutionRecord {
            command: command.command.clone(),
            purpose: command.purpose.clone(),
            status: CommandExecutionStatus::Failed,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(err.to_string()),
            next_action: Some(
                "Check that netsh is available and rerun from an Administrator terminal."
                    .to_owned(),
            ),
        },
    }
}

fn run_tunnel_loopback_test(
    bind: &str,
    key: &str,
    message: &str,
    timeout_ms: u64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind(bind)?;
    socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let local_addr = socket.local_addr()?;
    let envelope = seal_tunnel_payload(
        key,
        "loopback-test",
        1,
        current_epoch_ms(),
        message.as_bytes(),
    )?;
    let wire = serde_json::to_vec(&envelope)?;
    let sent = socket.send_to(&wire, local_addr)?;
    let mut buffer = vec![0u8; 65_535];
    let (received, peer) = socket.recv_from(&mut buffer)?;
    let received_envelope: TunnelEnvelope = serde_json::from_slice(&buffer[..received])?;
    let payload = open_tunnel_payload(key, &received_envelope)?;

    Ok(serde_json::json!({
        "status": "ok",
        "bind": local_addr.to_string(),
        "peer": peer.to_string(),
        "bytesSent": sent,
        "bytesReceived": received,
        "message": String::from_utf8_lossy(&payload.plaintext),
        "metadata": payload.metadata,
    }))
}

fn run_tunnel_listener(
    bind: &str,
    key: &str,
    max_packets: u16,
    timeout_ms: u64,
    echo: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind(bind)?;
    socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let mut buffer = vec![0u8; 65_535];
    let mut packets = Vec::new();

    for index in 0..max_packets {
        match socket.recv_from(&mut buffer) {
            Ok((received, peer)) => {
                let received_envelope: TunnelEnvelope =
                    serde_json::from_slice(&buffer[..received])?;
                let payload = open_tunnel_payload(key, &received_envelope)?;
                if echo {
                    let reply = seal_tunnel_payload(
                        key,
                        "echo",
                        index as u64 + 1,
                        current_epoch_ms(),
                        &payload.plaintext,
                    )?;
                    let wire = serde_json::to_vec(&reply)?;
                    socket.send_to(&wire, peer)?;
                }
                packets.push(serde_json::json!({
                    "peer": peer.to_string(),
                    "bytesReceived": received,
                    "message": String::from_utf8_lossy(&payload.plaintext),
                    "metadata": payload.metadata,
                }));
            }
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::WouldBlock
                        | ErrorKind::TimedOut
                        | ErrorKind::Interrupted
                        | ErrorKind::ConnectionReset
                ) =>
            {
                break;
            }
            Err(err) => return Err(err.into()),
        }
    }

    Ok(serde_json::json!({
        "status": if packets.is_empty() { "timeout" } else { "ok" },
        "bind": socket.local_addr()?.to_string(),
        "echo": echo,
        "packets": packets,
    }))
}

fn run_tunnel_send(
    bind: &str,
    peer: &str,
    key: &str,
    message: &str,
    timeout_ms: u64,
    wait_reply: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind(bind)?;
    socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let peer = peer.parse::<SocketAddr>()?;
    let envelope = seal_tunnel_payload(key, "game-udp", 1, current_epoch_ms(), message.as_bytes())?;
    let wire = serde_json::to_vec(&envelope)?;
    let sent = socket.send_to(&wire, peer)?;
    let reply = if wait_reply {
        let mut buffer = vec![0u8; 65_535];
        match socket.recv_from(&mut buffer) {
            Ok((received, reply_peer)) => {
                let reply_envelope: TunnelEnvelope = serde_json::from_slice(&buffer[..received])?;
                let payload = open_tunnel_payload(key, &reply_envelope)?;
                Some(serde_json::json!({
                    "peer": reply_peer.to_string(),
                    "bytesReceived": received,
                    "message": String::from_utf8_lossy(&payload.plaintext),
                    "metadata": payload.metadata,
                }))
            }
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::WouldBlock
                        | ErrorKind::TimedOut
                        | ErrorKind::Interrupted
                        | ErrorKind::ConnectionReset
                ) =>
            {
                None
            }
            Err(err) => return Err(err.into()),
        }
    } else {
        None
    };

    Ok(serde_json::json!({
        "status": if wait_reply && reply.is_none() { "sent-no-reply" } else { "ok" },
        "bind": socket.local_addr()?.to_string(),
        "peer": peer.to_string(),
        "bytesSent": sent,
        "reply": reply,
    }))
}

fn run_relay_udp_server(
    bind: &str,
    key: &str,
    room_id: &str,
    allowed_peers: &[String],
    max_packets: u16,
    timeout_ms: u64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind(bind)?;
    socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let allowed_peers = allowed_peers
        .iter()
        .filter(|peer| !peer.trim().is_empty())
        .cloned()
        .collect::<HashSet<_>>();
    let mut peer_endpoints = HashMap::<String, SocketAddr>::new();
    let mut packets = Vec::new();
    let mut forwarded_packets = 0u64;
    let mut dropped_packets = 0u64;
    let mut decrypted_packets = 0u64;
    let mut buffer = vec![0u8; 65_535];

    loop {
        if max_packets > 0 && packets.len() >= max_packets as usize {
            break;
        }
        match socket.recv_from(&mut buffer) {
            Ok((received, source)) => {
                let observed_at_ms = current_epoch_ms();
                let event = match relay_packet_event(
                    &socket,
                    key,
                    room_id,
                    &allowed_peers,
                    &mut peer_endpoints,
                    &buffer[..received],
                    received,
                    source,
                    observed_at_ms,
                ) {
                    Ok(event) => {
                        decrypted_packets += 1;
                        if event.get("status").and_then(serde_json::Value::as_str)
                            == Some("forwarded")
                        {
                            forwarded_packets += 1;
                        } else if event.get("status").and_then(serde_json::Value::as_str)
                            != Some("registered")
                        {
                            dropped_packets += 1;
                        }
                        event
                    }
                    Err(err) => {
                        dropped_packets += 1;
                        serde_json::json!({
                            "status": "dropped",
                            "reason": "invalid-relay-packet",
                            "source": source.to_string(),
                            "bytesReceived": received,
                            "observedAtMs": observed_at_ms,
                            "error": err.to_string(),
                        })
                    }
                };
                packets.push(event);
            }
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::WouldBlock
                        | ErrorKind::TimedOut
                        | ErrorKind::Interrupted
                        | ErrorKind::ConnectionReset
                ) =>
            {
                break;
            }
            Err(err) => return Err(err.into()),
        }
    }

    Ok(serde_json::json!({
        "status": if forwarded_packets > 0 {
            "ok"
        } else if packets.is_empty() {
            "timeout"
        } else {
            "no-forward"
        },
        "bind": socket.local_addr()?.to_string(),
        "roomId": room_id,
        "allowedPeerCount": allowed_peers.len(),
        "knownPeerCount": peer_endpoints.len(),
        "decryptedPackets": decrypted_packets,
        "forwardedPackets": forwarded_packets,
        "droppedPackets": dropped_packets,
        "packets": packets,
        "knownPeers": relay_known_peers(&peer_endpoints),
    }))
}

fn relay_packet_event(
    socket: &UdpSocket,
    key: &str,
    room_id: &str,
    allowed_peers: &HashSet<String>,
    peer_endpoints: &mut HashMap<String, SocketAddr>,
    wire: &[u8],
    received: usize,
    source: SocketAddr,
    observed_at_ms: u128,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let envelope: TunnelEnvelope = serde_json::from_slice(wire)?;
    let payload = open_tunnel_payload(key, &envelope)?;
    let request: serde_json::Value = serde_json::from_slice(&payload.plaintext)?;
    let request_room_id = request
        .get("room_id")
        .or_else(|| request.get("roomId"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let from_peer_id = request
        .get("from_peer_id")
        .or_else(|| request.get("fromPeerId"))
        .or_else(|| request.get("peer_id"))
        .or_else(|| request.get("peerId"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let to_peer_id = request
        .get("to_peer_id")
        .or_else(|| request.get("toPeerId"))
        .or_else(|| request.get("target_peer_id"))
        .or_else(|| request.get("targetPeerId"))
        .and_then(serde_json::Value::as_str);

    if request_room_id != room_id {
        return Ok(serde_json::json!({
            "status": "dropped",
            "reason": "room-mismatch",
            "source": source.to_string(),
            "roomId": request_room_id,
            "expectedRoomId": room_id,
            "bytesReceived": received,
            "observedAtMs": observed_at_ms,
            "packetKind": payload.metadata.packet_kind,
            "sequence": payload.metadata.sequence,
        }));
    }
    if from_peer_id.is_empty() {
        return Ok(serde_json::json!({
            "status": "dropped",
            "reason": "missing-from-peer",
            "source": source.to_string(),
            "bytesReceived": received,
            "observedAtMs": observed_at_ms,
            "packetKind": payload.metadata.packet_kind,
            "sequence": payload.metadata.sequence,
        }));
    }
    if !allowed_peers.is_empty() && !allowed_peers.contains(from_peer_id) {
        return Ok(serde_json::json!({
            "status": "dropped",
            "reason": "from-peer-not-allowed",
            "source": source.to_string(),
            "fromPeerId": from_peer_id,
            "bytesReceived": received,
            "observedAtMs": observed_at_ms,
            "packetKind": payload.metadata.packet_kind,
            "sequence": payload.metadata.sequence,
        }));
    }

    peer_endpoints.insert(from_peer_id.to_owned(), source);
    let Some(to_peer_id) = to_peer_id else {
        return Ok(serde_json::json!({
            "status": "registered",
            "source": source.to_string(),
            "fromPeerId": from_peer_id,
            "knownPeerCount": peer_endpoints.len(),
            "bytesReceived": received,
            "observedAtMs": observed_at_ms,
            "packetKind": payload.metadata.packet_kind,
            "sequence": payload.metadata.sequence,
        }));
    };
    if !allowed_peers.is_empty() && !allowed_peers.contains(to_peer_id) {
        return Ok(serde_json::json!({
            "status": "dropped",
            "reason": "target-peer-not-allowed",
            "source": source.to_string(),
            "fromPeerId": from_peer_id,
            "toPeerId": to_peer_id,
            "knownPeerCount": peer_endpoints.len(),
            "bytesReceived": received,
            "observedAtMs": observed_at_ms,
            "packetKind": payload.metadata.packet_kind,
            "sequence": payload.metadata.sequence,
        }));
    }
    let Some(target) = peer_endpoints.get(to_peer_id).copied() else {
        return Ok(serde_json::json!({
            "status": "dropped",
            "reason": "target-peer-unknown",
            "source": source.to_string(),
            "fromPeerId": from_peer_id,
            "toPeerId": to_peer_id,
            "knownPeerCount": peer_endpoints.len(),
            "bytesReceived": received,
            "observedAtMs": observed_at_ms,
            "packetKind": payload.metadata.packet_kind,
            "sequence": payload.metadata.sequence,
        }));
    };
    let sent = socket.send_to(wire, target)?;
    Ok(serde_json::json!({
        "status": "forwarded",
        "source": source.to_string(),
        "target": target.to_string(),
        "fromPeerId": from_peer_id,
        "toPeerId": to_peer_id,
        "knownPeerCount": peer_endpoints.len(),
        "bytesReceived": received,
        "bytesSent": sent,
        "observedAtMs": observed_at_ms,
        "packetKind": payload.metadata.packet_kind,
        "sequence": payload.metadata.sequence,
    }))
}

fn relay_known_peers(peer_endpoints: &HashMap<String, SocketAddr>) -> Vec<serde_json::Value> {
    let mut peers = peer_endpoints
        .iter()
        .map(|(peer_id, endpoint)| {
            serde_json::json!({
                "peerId": peer_id,
                "endpoint": endpoint.to_string(),
            })
        })
        .collect::<Vec<_>>();
    peers.sort_by(|left, right| {
        left.get("peerId")
            .and_then(serde_json::Value::as_str)
            .cmp(&right.get("peerId").and_then(serde_json::Value::as_str))
    });
    peers
}

fn run_relay_udp_loopback_test(
    bind: &str,
    key: &str,
    room_id: &str,
    message: &str,
    timeout_ms: u64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let relay = UdpSocket::bind(bind)?;
    relay.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let relay_addr = relay.local_addr()?;
    let peer_a = UdpSocket::bind("127.0.0.1:0")?;
    let peer_b = UdpSocket::bind("127.0.0.1:0")?;
    peer_b.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let allowed_peers = ["peer_a".to_owned(), "peer_b".to_owned()]
        .into_iter()
        .collect::<HashSet<_>>();
    let mut peer_endpoints = HashMap::<String, SocketAddr>::new();
    let mut relay_events = Vec::new();

    let register_b = relay_packet(key, "relay-register", 1, room_id, "peer_b", None, b"")?;
    let sent_register = peer_b.send_to(&register_b, relay_addr)?;
    let mut buffer = vec![0u8; 65_535];
    let (received_register, register_source) = relay.recv_from(&mut buffer)?;
    relay_events.push(relay_packet_event(
        &relay,
        key,
        room_id,
        &allowed_peers,
        &mut peer_endpoints,
        &buffer[..received_register],
        received_register,
        register_source,
        current_epoch_ms(),
    )?);

    let forward = relay_packet(
        key,
        "relay-udp-forward",
        2,
        room_id,
        "peer_a",
        Some("peer_b"),
        message.as_bytes(),
    )?;
    let sent_forward = peer_a.send_to(&forward, relay_addr)?;
    let (received_forward, forward_source) = relay.recv_from(&mut buffer)?;
    relay_events.push(relay_packet_event(
        &relay,
        key,
        room_id,
        &allowed_peers,
        &mut peer_endpoints,
        &buffer[..received_forward],
        received_forward,
        forward_source,
        current_epoch_ms(),
    )?);

    let (delivered_bytes, delivered_source) = peer_b.recv_from(&mut buffer)?;
    let delivered_envelope: TunnelEnvelope = serde_json::from_slice(&buffer[..delivered_bytes])?;
    let delivered_payload = open_tunnel_payload(key, &delivered_envelope)?;
    let delivered_request: serde_json::Value =
        serde_json::from_slice(&delivered_payload.plaintext)?;
    let delivered_message = delivered_request
        .get("bytes")
        .and_then(serde_json::Value::as_str)
        .map(|encoded| STANDARD_NO_PAD.decode(encoded.as_bytes()))
        .transpose()
        .map_err(|err| invalid_input(format!("invalid relay payload bytes: {err}")))?
        .unwrap_or_default();

    Ok(serde_json::json!({
        "status": if delivered_message == message.as_bytes() { "ok" } else { "mismatch" },
        "relay": relay_addr.to_string(),
        "peerA": peer_a.local_addr()?.to_string(),
        "peerB": peer_b.local_addr()?.to_string(),
        "bytesSentToRegister": sent_register,
        "bytesSentToForward": sent_forward,
        "deliveredBytes": delivered_bytes,
        "deliveredFrom": delivered_source.to_string(),
        "deliveredMessage": String::from_utf8_lossy(&delivered_message),
        "relayEvents": relay_events,
        "knownPeers": relay_known_peers(&peer_endpoints),
    }))
}

fn relay_packet(
    key: &str,
    packet_kind: &str,
    sequence: u64,
    room_id: &str,
    from_peer_id: &str,
    to_peer_id: Option<&str>,
    payload: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let body = serde_json::json!({
        "schemaVersion": 1,
        "room_id": room_id,
        "from_peer_id": from_peer_id,
        "to_peer_id": to_peer_id,
        "bytes": STANDARD_NO_PAD.encode(payload),
        "sentAtMs": current_epoch_ms(),
    });
    let envelope = seal_tunnel_payload(
        key,
        packet_kind,
        sequence,
        current_epoch_ms(),
        serde_json::to_string(&body)?.as_bytes(),
    )?;
    serde_json::to_vec(&envelope).map_err(Into::into)
}

fn run_p2p_handshake_loopback_test(
    bind: &str,
    room_id: &str,
    peer_id: &str,
    responder_peer_id: &str,
    virtual_ip: Ipv4Addr,
    key: &str,
    timeout_ms: u64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind(bind)?;
    socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let local_addr = socket.local_addr()?;
    let started_at_ms = current_epoch_ms();
    let hello = create_p2p_handshake_hello(
        room_id,
        peer_id,
        virtual_ip,
        local_addr.to_string(),
        random_nonce(),
        started_at_ms,
    );
    let hello_bytes = serde_json::to_vec(&hello)?;
    let hello_envelope =
        seal_tunnel_payload(key, "p2p-handshake-hello", 1, started_at_ms, &hello_bytes)?;
    socket.send_to(&serde_json::to_vec(&hello_envelope)?, local_addr)?;

    let mut buffer = vec![0u8; 65_535];
    let (hello_received, hello_peer) = socket.recv_from(&mut buffer)?;
    let received_hello_envelope: TunnelEnvelope =
        serde_json::from_slice(&buffer[..hello_received])?;
    let received_hello_payload = open_tunnel_payload(key, &received_hello_envelope)?;
    let received_hello: P2pHandshakeHello =
        serde_json::from_slice(&received_hello_payload.plaintext)?;

    let ack = create_p2p_handshake_ack(
        &received_hello,
        responder_peer_id,
        hello_peer.to_string(),
        current_epoch_ms(),
    );
    let ack_bytes = serde_json::to_vec(&ack)?;
    let ack_envelope =
        seal_tunnel_payload(key, "p2p-handshake-ack", 2, current_epoch_ms(), &ack_bytes)?;
    socket.send_to(&serde_json::to_vec(&ack_envelope)?, hello_peer)?;

    let (ack_received, ack_peer) = socket.recv_from(&mut buffer)?;
    let received_ack_envelope: TunnelEnvelope = serde_json::from_slice(&buffer[..ack_received])?;
    let received_ack_payload = open_tunnel_payload(key, &received_ack_envelope)?;
    let received_ack: P2pHandshakeAck = serde_json::from_slice(&received_ack_payload.plaintext)?;
    let finished_at_ms = current_epoch_ms();

    Ok(serde_json::json!({
        "status": if received_ack.accepted { "ok" } else { "rejected" },
        "roomId": room_id,
        "peerId": peer_id,
        "responderPeerId": received_ack.responder_peer_id,
        "localEndpoint": local_addr.to_string(),
        "observedEndpoint": received_ack.observed_endpoint,
        "ackPeer": ack_peer.to_string(),
        "virtualIp": virtual_ip.to_string(),
        "nonceMatched": received_ack.nonce == hello.nonce,
        "latencyMs": finished_at_ms.saturating_sub(started_at_ms),
        "helloBytes": hello_received,
        "ackBytes": ack_received,
    }))
}

fn run_p2p_handshake_listener(
    bind: &str,
    key: &str,
    responder_peer_id: &str,
    max_packets: u16,
    timeout_ms: u64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind(bind)?;
    socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let local_addr = socket.local_addr()?;
    let mut buffer = vec![0u8; 65_535];
    let mut handshakes = Vec::new();
    let mut ignored_packets = Vec::new();

    while handshakes.len() < max_packets as usize {
        match socket.recv_from(&mut buffer) {
            Ok((received, peer)) => {
                let envelope: TunnelEnvelope = match serde_json::from_slice(&buffer[..received]) {
                    Ok(value) => value,
                    Err(_) => {
                        ignored_packets.push(serde_json::json!({
                            "peer": peer.to_string(),
                            "bytes": received,
                            "reason": "not-tunnel-envelope",
                        }));
                        continue;
                    }
                };
                let payload = match open_tunnel_payload(key, &envelope) {
                    Ok(value) => value,
                    Err(_) => {
                        ignored_packets.push(serde_json::json!({
                            "peer": peer.to_string(),
                            "bytes": received,
                            "reason": "decrypt-failed",
                        }));
                        continue;
                    }
                };
                if payload.metadata.packet_kind != "p2p-handshake-hello" {
                    ignored_packets.push(serde_json::json!({
                        "peer": peer.to_string(),
                        "bytes": received,
                        "reason": "unexpected-packet-kind",
                        "packetKind": payload.metadata.packet_kind,
                    }));
                    continue;
                }
                let hello: P2pHandshakeHello = serde_json::from_slice(&payload.plaintext)?;
                let ack = create_p2p_handshake_ack(
                    &hello,
                    responder_peer_id,
                    peer.to_string(),
                    current_epoch_ms(),
                );
                let ack_bytes = serde_json::to_vec(&ack)?;
                let ack_envelope = seal_tunnel_payload(
                    key,
                    "p2p-handshake-ack",
                    handshakes.len() as u64 + 1,
                    current_epoch_ms(),
                    &ack_bytes,
                )?;
                let sent = socket.send_to(&serde_json::to_vec(&ack_envelope)?, peer)?;
                handshakes.push(serde_json::json!({
                    "peer": peer.to_string(),
                    "roomId": hello.room_id,
                    "peerId": hello.peer_id,
                    "virtualIp": hello.virtual_ip,
                    "listenEndpoint": hello.listen_endpoint,
                    "nonce": hello.nonce,
                    "helloBytes": received,
                    "ackBytes": sent,
                }));
            }
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::WouldBlock
                        | ErrorKind::TimedOut
                        | ErrorKind::Interrupted
                        | ErrorKind::ConnectionReset
                ) =>
            {
                break;
            }
            Err(err) => return Err(err.into()),
        }
    }

    Ok(serde_json::json!({
        "status": if handshakes.is_empty() { "timeout" } else { "ok" },
        "bind": local_addr.to_string(),
        "responderPeerId": responder_peer_id,
        "handshakes": handshakes,
        "ignoredPackets": ignored_packets,
    }))
}

fn run_p2p_handshake_send(
    bind: &str,
    peer: &str,
    room_id: &str,
    peer_id: &str,
    virtual_ip: Ipv4Addr,
    key: &str,
    timeout_ms: u64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind(bind)?;
    socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let local_addr = socket.local_addr()?;
    let peer = peer.parse::<SocketAddr>()?;
    let started_at_ms = current_epoch_ms();
    let hello = create_p2p_handshake_hello(
        room_id,
        peer_id,
        virtual_ip,
        local_addr.to_string(),
        random_nonce(),
        started_at_ms,
    );
    let hello_bytes = serde_json::to_vec(&hello)?;
    let hello_envelope =
        seal_tunnel_payload(key, "p2p-handshake-hello", 1, started_at_ms, &hello_bytes)?;
    let sent = socket.send_to(&serde_json::to_vec(&hello_envelope)?, peer)?;

    let mut buffer = vec![0u8; 65_535];
    let (received, ack_peer) = socket.recv_from(&mut buffer)?;
    let ack_envelope: TunnelEnvelope = serde_json::from_slice(&buffer[..received])?;
    let payload = open_tunnel_payload(key, &ack_envelope)?;
    if payload.metadata.packet_kind != "p2p-handshake-ack" {
        return Err(invalid_input(format!(
            "unexpected P2P handshake packet kind `{}`",
            payload.metadata.packet_kind
        )));
    }
    let ack: P2pHandshakeAck = serde_json::from_slice(&payload.plaintext)?;
    let finished_at_ms = current_epoch_ms();

    Ok(serde_json::json!({
        "status": if ack.accepted && ack.nonce == hello.nonce { "ok" } else { "rejected" },
        "bind": local_addr.to_string(),
        "peer": peer.to_string(),
        "ackPeer": ack_peer.to_string(),
        "roomId": room_id,
        "peerId": peer_id,
        "responderPeerId": ack.responder_peer_id,
        "virtualIp": virtual_ip.to_string(),
        "observedEndpoint": ack.observed_endpoint,
        "nonceMatched": ack.nonce == hello.nonce,
        "latencyMs": finished_at_ms.saturating_sub(started_at_ms),
        "helloBytes": sent,
        "ackBytes": received,
    }))
}

fn check_runtime_coordination_monitor(
    monitor: &RuntimeCoordinationMonitor,
    room_id: &str,
    peer_id: &str,
    virtual_ip: Ipv4Addr,
) -> Result<RuntimeCoordinationMonitorReport, Box<dyn std::error::Error>> {
    let subnet = monitor_subnet_for_virtual_ip(virtual_ip);
    let checked_at_ms = current_epoch_ms();
    let (source, view) = if let Some(store_path) = monitor.store_path.as_deref() {
        let store = load_coordination_store_or_default(store_path)?;
        (
            format!("coordination-store:{store_path}"),
            serde_json::to_value(lai_core::coordination_room_view(
                &store,
                room_id.to_owned(),
                peer_id.to_owned(),
                subnet,
                checked_at_ms,
            ))?,
        )
    } else if let Some(server) = monitor.server.as_deref() {
        (
            format!("coordination-http:{server}"),
            coordination_http_room_view(server, room_id, peer_id, subnet)?,
        )
    } else {
        return Ok(RuntimeCoordinationMonitorReport {
            status: "disabled".to_owned(),
            source: "none".to_owned(),
            room_id: room_id.to_owned(),
            peer_id: peer_id.to_owned(),
            peer_present: true,
            room_present: true,
            checked_at_ms,
            detail: "Coordination monitor has no store or server configured.".to_owned(),
        });
    };

    let members = view
        .get("members")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let peer_present = members
        .iter()
        .any(|member| member.get("peer_id").and_then(serde_json::Value::as_str) == Some(peer_id));
    let room_present = !members.is_empty();
    let status = if !room_present {
        "room-closed"
    } else if !peer_present {
        "peer-removed"
    } else {
        "ok"
    }
    .to_owned();
    let detail = match status.as_str() {
        "room-closed" => {
            "Coordination room is missing or empty; stopping local runtime.".to_owned()
        }
        "peer-removed" => {
            "Local peer is no longer present in coordination room; stopping local runtime."
                .to_owned()
        }
        _ => "Local peer is still present in coordination room.".to_owned(),
    };

    Ok(RuntimeCoordinationMonitorReport {
        status,
        source,
        room_id: room_id.to_owned(),
        peer_id: peer_id.to_owned(),
        peer_present,
        room_present,
        checked_at_ms,
        detail,
    })
}

fn monitor_subnet_for_virtual_ip(virtual_ip: Ipv4Addr) -> Ipv4Subnet {
    let octets = virtual_ip.octets();
    Ipv4Subnet {
        network: Ipv4Addr::new(octets[0], octets[1], octets[2], 0),
        prefix: 24,
    }
}

fn run_room_runtime(
    plan: &RoomRuntimePlan,
    key: &str,
    duration_ms: u64,
    observe_file: Option<&str>,
    snapshot_out: Option<&str>,
    packet_io_backend: &str,
    forward_raw_ipv4: bool,
    self_probe: bool,
    capture_self_probe: bool,
    forward_self_probe: bool,
    inject_self_probe: bool,
    explicit_inject_target: Option<&str>,
    heartbeat_interval_ms: u64,
    peer_timeout_ms: u64,
    stop_file: Option<&str>,
    snapshot_interval_ms: Option<u64>,
    coordination_monitor: Option<RuntimeCoordinationMonitor>,
    packet_io_probe_options: &RuntimePacketIoProbeOptions,
    wintun_runtime: bool,
    expected_broadcast_ports: Vec<u16>,
    expected_game_ports: Vec<u16>,
    max_broadcast_packets_per_second: u16,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let started_at_ms = current_epoch_ms();
    let started_at = Instant::now();
    let packet_io_plan = lai_core::create_virtual_packet_io_plan(
        "LocalAreaInterconnection",
        packet_io_backend,
        1420,
    );
    let packet_io_probe = runtime_packet_io_probe(packet_io_backend, packet_io_probe_options);
    let mut wintun_packet_io = None;
    let mut wintun_runtime_open = serde_json::json!({
        "enabled": wintun_runtime,
        "status": if wintun_runtime { "not-opened" } else { "disabled" },
    });
    if packet_io_backend == "wintun" && wintun_runtime {
        match lai_core::open_wintun_packet_io_session(lai_core::WintunPacketIoConfig {
            adapter_name: packet_io_probe_options.wintun_adapter_name.clone(),
            ring_capacity: packet_io_probe_options.wintun_ring_capacity,
        }) {
            Ok(session) => {
                wintun_runtime_open = serde_json::json!({
                    "enabled": true,
                    "status": "session-opened",
                    "adapterName": packet_io_probe_options.wintun_adapter_name.clone(),
                    "ringCapacity": packet_io_probe_options.wintun_ring_capacity,
                });
                wintun_packet_io = Some(session);
            }
            Err(report) => {
                wintun_runtime_open = serde_json::json!({
                    "enabled": true,
                    "status": report.status,
                    "report": report,
                });
            }
        }
    }
    let tunnel_socket = UdpSocket::bind(&plan.tunnel.bind_endpoint)?;
    tunnel_socket.set_read_timeout(Some(Duration::from_millis(25)))?;
    let tunnel_endpoint = tunnel_socket.local_addr()?;
    let mut bytes_sent = 0u64;
    let mut bytes_received = 0u64;
    let mut connected_peer_count = 0u16;
    let mut last_error = None;

    let mut capture_sockets = Vec::new();
    for binding in &plan.capture_ports {
        if binding.protocol != "udp" {
            continue;
        }
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", binding.port))?;
        socket.set_broadcast(true)?;
        socket.set_read_timeout(Some(Duration::from_millis(10)))?;
        let actual_port = socket.local_addr()?.port();
        capture_sockets.push((actual_port, binding.purpose.clone(), socket));
    }
    let actual_game_ports = capture_sockets
        .iter()
        .filter(|(_, purpose, _)| purpose == "game-traffic")
        .map(|(port, _, _)| *port)
        .collect::<Vec<_>>();
    let actual_broadcast_ports = capture_sockets
        .iter()
        .filter(|(_, purpose, _)| purpose == "broadcast-discovery")
        .map(|(port, _, _)| *port)
        .collect::<Vec<_>>();
    let runtime_expected_game_ports =
        normalize_runtime_expected_ports(expected_game_ports, &actual_game_ports);
    let runtime_expected_broadcast_ports =
        normalize_runtime_expected_ports(expected_broadcast_ports, &actual_broadcast_ports);
    let runtime_cleanup_plan = lai_core::create_windows_runtime_cleanup_plan_with_routes(
        plan.room_id.clone(),
        plan.local_peer_id.clone(),
        plan.local_virtual_ip,
        Some(runtime_subnet_from_local_ip(plan.local_virtual_ip)),
        packet_io_probe_options.wintun_adapter_name.clone(),
        packet_io_backend.to_owned(),
        false,
        false,
    );
    let mut broadcast_gate = lai_core::BroadcastForwardGate::new(
        lai_core::BroadcastPolicy::with_limit(
            runtime_subnet_from_local_ip(plan.local_virtual_ip),
            runtime_expected_broadcast_ports.clone(),
            max_broadcast_packets_per_second,
        ),
        started_at_ms,
    );
    let mut broadcast_forward_events = Vec::new();
    let forward_targets_by_port = runtime_forward_targets(
        plan,
        &actual_broadcast_ports,
        forward_self_probe,
        tunnel_endpoint,
    )?;
    let inject_receiver = if inject_self_probe {
        let socket = UdpSocket::bind("127.0.0.1:0")?;
        socket.set_read_timeout(Some(Duration::from_millis(5)))?;
        Some(socket)
    } else {
        None
    };
    let inject_target = if let Some(target) = explicit_inject_target {
        Some(target.parse::<SocketAddr>().map_err(|err| {
            invalid_input(format!("invalid runtime inject target `{target}`: {err}"))
        })?)
    } else {
        inject_receiver
            .as_ref()
            .map(UdpSocket::local_addr)
            .transpose()?
    };
    let heartbeat_targets = runtime_heartbeat_targets(plan, self_probe, tunnel_endpoint)?;
    let heartbeat_interval =
        (heartbeat_interval_ms > 0).then(|| Duration::from_millis(heartbeat_interval_ms));
    let peer_timeout = (peer_timeout_ms > 0).then(|| Duration::from_millis(peer_timeout_ms));
    let deadline = (duration_ms > 0).then(|| started_at + Duration::from_millis(duration_ms));
    let mut next_heartbeat_at = started_at;
    let mut next_snapshot_at =
        snapshot_interval_ms.map(|interval_ms| started_at + Duration::from_millis(interval_ms));
    let coordination_monitor_interval = coordination_monitor
        .as_ref()
        .map(|monitor| Duration::from_millis(monitor.interval_ms.max(1)));
    let mut next_coordination_monitor_at = coordination_monitor
        .as_ref()
        .map(|_| started_at + Duration::from_millis(1));
    let mut heartbeat_packets = Vec::new();
    let mut heartbeat_ack_packets = Vec::new();
    let mut coordination_monitor_reports = Vec::new();
    let mut snapshot_write_count = 0u32;
    let mut last_peer_packet_at = None;
    let mut peer_timed_out = false;
    let stop_reason: &str;

    if capture_self_probe {
        for (_, _, socket) in &capture_sockets {
            let client = UdpSocket::bind("127.0.0.1:0")?;
            client.send_to(
                b"runtime-capture-probe",
                loopback_target(socket.local_addr()?),
            )?;
        }
    }

    let mut tunnel_packets = Vec::new();
    let mut capture_summaries = Vec::new();
    let mut observation_lines = Vec::new();
    let mut forwarded_packets = Vec::new();
    let mut injected_packets = Vec::new();
    let mut injected_received_packets = Vec::new();
    let mut raw_virtual_packets = Vec::new();
    let mut wintun_runtime_received_packets = Vec::new();
    let mut wintun_runtime_sent_packets = Vec::new();
    let mut wintun_runtime_errors = Vec::new();
    let mut buffer = vec![0u8; 65_535];

    loop {
        let now = Instant::now();
        if let Some(deadline) = deadline {
            if now >= deadline {
                stop_reason = "duration";
                break;
            }
        }
        if let Some(path) = stop_file {
            if Path::new(path).exists() {
                stop_reason = "stop-file";
                break;
            }
        }

        if heartbeat_interval.is_some() && now >= next_heartbeat_at {
            for target in &heartbeat_targets {
                let sequence = heartbeat_packets.len() as u64 + 1;
                let sent_at_ms = current_epoch_ms();
                let heartbeat = serde_json::json!({
                    "room_id": plan.room_id,
                    "peer_id": plan.local_peer_id,
                    "virtual_ip": plan.local_virtual_ip,
                    "kind": "runtime-heartbeat",
                    "sequence": sequence,
                    "sent_at_ms": sent_at_ms,
                });
                let envelope = seal_tunnel_payload(
                    key,
                    "runtime-heartbeat",
                    sequence,
                    sent_at_ms,
                    serde_json::to_string(&heartbeat)?.as_bytes(),
                )?;
                let wire = serde_json::to_vec(&envelope)?;
                match tunnel_socket.send_to(&wire, target) {
                    Ok(sent) => {
                        bytes_sent += sent as u64;
                        heartbeat_packets.push(serde_json::json!({
                            "target": target.to_string(),
                            "bytesSent": sent,
                            "sequence": sequence,
                            "sentAtMs": sent_at_ms,
                        }));
                    }
                    Err(err) => {
                        last_error = Some(format!(
                            "Failed to send runtime heartbeat to {target}: {err}"
                        ));
                    }
                }
            }
            if let Some(interval) = heartbeat_interval {
                next_heartbeat_at = now + interval;
            }
        }

        if let (Some(path), Some(interval_ms), Some(next_snapshot)) =
            (snapshot_out, snapshot_interval_ms, next_snapshot_at)
        {
            if now >= next_snapshot {
                let tunnel_endpoint_text = tunnel_endpoint.to_string();
                let tick = serde_json::json!({
                    "status": "running",
                    "startedAtMs": started_at_ms,
                    "updatedAtMs": current_epoch_ms(),
                    "actualTunnelEndpoint": tunnel_endpoint_text.clone(),
                    "bytesSent": bytes_sent,
                    "bytesReceived": bytes_received,
                    "heartbeatPacketsSent": heartbeat_packets.len(),
                    "tunnelPacketCount": tunnel_packets.len(),
                    "packetCaptureCount": capture_summaries.len(),
                    "forwardedPacketCount": forwarded_packets.len(),
                    "broadcastForwardReport": lai_core::create_broadcast_forward_report(
                        broadcast_gate.policy(),
                        broadcast_forward_events.clone(),
                    ),
                    "injectedPacketCount": injected_packets.len(),
                    "wintunRuntimeReceivedPacketCount": wintun_runtime_received_packets.len(),
                    "wintunRuntimeSentPacketCount": wintun_runtime_sent_packets.len(),
                    "packetIoProbe": packet_io_probe.clone(),
                    "adapterWriteStatus": packet_io_probe["adapterWriteStatus"].clone(),
                    "adapterReadStatus": packet_io_probe["adapterReadStatus"].clone(),
                    "wintunRuntime": wintun_runtime_open.clone(),
                    "runtimeCleanupPlan": runtime_cleanup_plan.clone(),
                    "runtimePeerSummaries": runtime_peer_summaries(
                        plan,
                        &[],
                        &tunnel_packets,
                        &forwarded_packets,
                        &heartbeat_packets,
                        &heartbeat_ack_packets,
                        if connected_peer_count > 0 { Some("p2p") } else { None },
                        Some(tunnel_endpoint_text.as_str()),
                    ),
                    "coordinationMonitorReports": coordination_monitor_reports.clone(),
                    "lastError": last_error.clone(),
                });
                write_json_file(path, &tick)?;
                snapshot_write_count += 1;
                next_snapshot_at = Some(now + Duration::from_millis(interval_ms));
            }
        }

        if let (Some(monitor), Some(interval), Some(next_check)) = (
            coordination_monitor.as_ref(),
            coordination_monitor_interval,
            next_coordination_monitor_at,
        ) {
            if now >= next_check {
                let report = check_runtime_coordination_monitor(
                    monitor,
                    &plan.room_id,
                    &plan.local_peer_id,
                    plan.local_virtual_ip,
                );
                let stop_status = report.as_ref().ok().map(|report| report.status.clone());
                match report {
                    Ok(report) => {
                        let should_stop =
                            matches!(report.status.as_str(), "room-closed" | "peer-removed");
                        if should_stop {
                            last_error = Some(report.detail.clone());
                            coordination_monitor_reports.push(serde_json::to_value(&report)?);
                            stop_reason = match report.status.as_str() {
                                "room-closed" => "coordination-room-closed",
                                "peer-removed" => "coordination-peer-removed",
                                _ => "coordination-monitor",
                            };
                            break;
                        }
                        coordination_monitor_reports.push(serde_json::to_value(report)?);
                    }
                    Err(err) => {
                        last_error = Some(format!("Coordination monitor failed: {err}"));
                        coordination_monitor_reports.push(serde_json::json!({
                            "status": "error",
                            "room_id": plan.room_id,
                            "peer_id": plan.local_peer_id,
                            "checked_at_ms": current_epoch_ms(),
                            "detail": err.to_string(),
                        }));
                    }
                }
                next_coordination_monitor_at = Some(now + interval);
                if matches!(
                    stop_status.as_deref(),
                    Some("room-closed") | Some("peer-removed")
                ) {
                    continue;
                }
            }
        }

        if !plan.peers.is_empty() {
            if let Some(timeout) = peer_timeout {
                let timed_out = last_peer_packet_at
                    .map(|last_seen| now.saturating_duration_since(last_seen) > timeout)
                    .unwrap_or_else(|| now.saturating_duration_since(started_at) > timeout);
                if timed_out && !peer_timed_out {
                    peer_timed_out = true;
                    last_error = Some(format!(
                        "No runtime tunnel packets were received from configured peers within {peer_timeout_ms}ms."
                    ));
                }
            }
        }

        match tunnel_socket.recv_from(&mut buffer) {
            Ok((received, peer)) => {
                bytes_received += received as u64;
                match serde_json::from_slice::<TunnelEnvelope>(&buffer[..received])
                    .ok()
                    .and_then(|envelope| open_tunnel_payload(key, &envelope).ok())
                {
                    Some(payload) => {
                        let received_at_ms = current_epoch_ms();
                        last_peer_packet_at = Some(Instant::now());
                        peer_timed_out = false;
                        connected_peer_count = connected_peer_count.max(1);
                        if payload.metadata.packet_kind == "runtime-heartbeat" {
                            match runtime_heartbeat_ack_payload(
                                plan,
                                &payload.plaintext,
                                received_at_ms,
                            ) {
                                Ok((ack_payload, acked_sequence, heartbeat_sent_at_ms)) => {
                                    let ack_sent_at_ms = current_epoch_ms();
                                    let envelope = seal_tunnel_payload(
                                        key,
                                        "runtime-heartbeat-ack",
                                        acked_sequence,
                                        ack_sent_at_ms,
                                        serde_json::to_string(&ack_payload)?.as_bytes(),
                                    )?;
                                    let wire = serde_json::to_vec(&envelope)?;
                                    match tunnel_socket.send_to(&wire, peer) {
                                        Ok(sent) => {
                                            bytes_sent += sent as u64;
                                            heartbeat_ack_packets.push(serde_json::json!({
                                                "direction": "sent",
                                                "target": peer.to_string(),
                                                "bytesSent": sent,
                                                "ackedSequence": acked_sequence,
                                                "heartbeatSentAtMs": heartbeat_sent_at_ms,
                                                "receivedAtMs": received_at_ms,
                                                "sentAtMs": ack_sent_at_ms,
                                            }));
                                        }
                                        Err(err) => {
                                            last_error = Some(format!(
                                                "Failed to send runtime heartbeat ack to {peer}: {err}"
                                            ));
                                        }
                                    }
                                }
                                Err(err) => {
                                    last_error =
                                        Some(format!("Failed to decode runtime heartbeat: {err}"));
                                }
                            }
                        } else if payload.metadata.packet_kind == "runtime-heartbeat-ack" {
                            match runtime_heartbeat_ack_observation(
                                peer,
                                received,
                                &payload.plaintext,
                                received_at_ms,
                            ) {
                                Ok(ack) => heartbeat_ack_packets.push(ack),
                                Err(err) => {
                                    last_error = Some(format!(
                                        "Failed to decode runtime heartbeat ack: {err}"
                                    ));
                                }
                            }
                        }
                        if payload.metadata.packet_kind == "runtime-udp-forward"
                            || payload.metadata.packet_kind == "runtime-ipv4-forward"
                        {
                            match runtime_forward_payload_data(&payload.plaintext) {
                                Ok(forward_data) => {
                                    if let Some(raw_packet) = &forward_data.raw_ipv4_packet {
                                        let raw_observation =
                                            lai_core::udp_observation_from_virtual_packet(
                                                raw_packet,
                                            );
                                        observation_lines.push(
                                            lai_core::packet_observation_line_from_udp_forward(
                                                &raw_observation,
                                            ),
                                        );
                                        append_observation_lines(
                                            observe_file,
                                            std::slice::from_ref(&raw_observation),
                                        )?;
                                        capture_summaries.push(PacketCaptureSummary {
                                            protocol: "udp".to_owned(),
                                            source_ip: raw_packet.source_ip,
                                            destination_ip: raw_packet.destination_ip,
                                            destination_port: raw_packet.destination_port,
                                            direction: "virtual-adapter".to_owned(),
                                            broadcast: raw_packet.broadcast,
                                            packet_count: 1,
                                            bytes: raw_packet.payload.len() as u32,
                                        });
                                        raw_virtual_packets.push(serde_json::json!({
                                            "protocol": "udp",
                                            "sourceIp": raw_packet.source_ip,
                                            "destinationIp": raw_packet.destination_ip,
                                            "sourcePort": raw_packet.source_port,
                                            "destinationPort": raw_packet.destination_port,
                                            "payloadBytes": raw_packet.payload.len(),
                                            "broadcast": raw_packet.broadcast,
                                        }));
                                    } else if let Some(tcp_packet) = &forward_data.raw_tcp_packet {
                                        let raw_observation =
                                            lai_core::tcp_observation_from_virtual_packet(
                                                tcp_packet,
                                            );
                                        observation_lines.push(
                                            lai_core::packet_observation_line_from_transport(
                                                "tcp",
                                                &raw_observation,
                                            ),
                                        );
                                        append_observation_text_lines(
                                            observe_file,
                                            &[lai_core::packet_observation_line_from_transport(
                                                "tcp",
                                                &raw_observation,
                                            )],
                                        )?;
                                        capture_summaries.push(PacketCaptureSummary {
                                            protocol: "tcp".to_owned(),
                                            source_ip: tcp_packet.source_ip,
                                            destination_ip: tcp_packet.destination_ip,
                                            destination_port: tcp_packet.destination_port,
                                            direction: "virtual-adapter".to_owned(),
                                            broadcast: false,
                                            packet_count: 1,
                                            bytes: tcp_packet.payload.len() as u32,
                                        });
                                        raw_virtual_packets.push(serde_json::json!({
                                            "protocol": "tcp",
                                            "sourceIp": tcp_packet.source_ip,
                                            "destinationIp": tcp_packet.destination_ip,
                                            "sourcePort": tcp_packet.source_port,
                                            "destinationPort": tcp_packet.destination_port,
                                            "payloadBytes": tcp_packet.payload.len(),
                                            "flags": tcp_packet.flags,
                                            "broadcast": false,
                                        }));
                                    } else if let Some(summary) = &forward_data.raw_ipv4_summary {
                                        raw_virtual_packets.push(serde_json::json!({
                                            "protocol": summary.protocol.clone(),
                                            "protocolNumber": summary.protocol_number,
                                            "sourceIp": summary.source_ip,
                                            "destinationIp": summary.destination_ip,
                                            "payloadBytes": summary.payload_bytes,
                                            "packetBytes": summary.packet_bytes,
                                            "broadcast": summary.broadcast,
                                        }));
                                    }
                                    if let (Some(session), Some(raw_bytes), Some(summary)) = (
                                        wintun_packet_io.as_mut(),
                                        forward_data.raw_ipv4_packet_bytes.as_ref(),
                                        forward_data.raw_ipv4_summary.as_ref(),
                                    ) {
                                        match session.send_ipv4_packet(raw_bytes) {
                                            Ok(bytes_sent_to_adapter) => {
                                                wintun_runtime_sent_packets.push(
                                                    serde_json::json!({
                                                        "direction": "tunnel-to-adapter",
                                                        "protocol": summary.protocol.clone(),
                                                        "bytesSent": bytes_sent_to_adapter,
                                                        "sourceIp": summary.source_ip,
                                                        "destinationIp": summary.destination_ip,
                                                        "destinationPort": summary.destination_port,
                                                        "broadcast": summary.broadcast,
                                                    }),
                                                );
                                            }
                                            Err(err) => {
                                                let message = format!(
                                                    "Failed to write raw IPv4 packet to Wintun: {err}"
                                                );
                                                last_error = Some(message.clone());
                                                wintun_runtime_errors.push(message);
                                            }
                                        }
                                    }
                                    if let Some(target) = inject_target.filter(|_| {
                                        forward_data.raw_ipv4_summary.is_none()
                                            || forward_data.raw_ipv4_packet.is_some()
                                    }) {
                                        let injector = UdpSocket::bind("127.0.0.1:0")?;
                                        match injector.send_to(&forward_data.udp_payload, target) {
                                            Ok(sent) => {
                                                injected_packets.push(serde_json::json!({
                                                    "target": target.to_string(),
                                                    "bytesSent": sent,
                                                    "rawIpv4PacketBytes": forward_data.raw_ipv4_packet_bytes.as_ref().map(Vec::len),
                                                }));
                                            }
                                            Err(err) => {
                                                last_error = Some(format!(
                                                    "Failed to inject runtime UDP forward payload to {target}: {err}"
                                                ));
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    last_error = Some(format!(
                                        "Failed to decode runtime UDP forward payload: {err}"
                                    ));
                                }
                            }
                        }
                        tunnel_packets.push(serde_json::json!({
                            "peer": peer.to_string(),
                            "bytes": received,
                            "kind": payload.metadata.packet_kind,
                            "sequence": payload.metadata.sequence,
                            "sentAtMs": payload.metadata.sent_at_ms,
                            "receivedAtMs": received_at_ms,
                        }));
                    }
                    None => {
                        last_error = Some("Failed to decrypt one tunnel packet.".to_owned());
                    }
                }
            }
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::WouldBlock
                        | ErrorKind::TimedOut
                        | ErrorKind::Interrupted
                        | ErrorKind::ConnectionReset
                ) => {}
            Err(err) => return Err(err.into()),
        }

        for (port, purpose, socket) in &capture_sockets {
            match socket.recv_from(&mut buffer) {
                Ok((received, source)) => {
                    let destination = socket.local_addr()?;
                    let broadcast = runtime_expected_broadcast_ports.contains(port)
                        || purpose == "broadcast-discovery";
                    let observation = UdpForwardObservation {
                        source,
                        destination,
                        bytes: received,
                        broadcast,
                        direction: "inbound".to_owned(),
                    };
                    observation_lines.push(lai_core::packet_observation_line_from_udp_forward(
                        &observation,
                    ));
                    append_observation_lines(observe_file, std::slice::from_ref(&observation))?;
                    capture_summaries.push(PacketCaptureSummary {
                        protocol: "udp".to_owned(),
                        source_ip: socket_addr_ipv4(source),
                        destination_ip: socket_addr_ipv4(destination),
                        destination_port: destination.port(),
                        direction: "inbound".to_owned(),
                        broadcast,
                        packet_count: 1,
                        bytes: received as u32,
                    });
                    if broadcast {
                        let targets = forward_targets_by_port
                            .iter()
                            .find(|(forward_port, _)| forward_port == port)
                            .map(|(_, targets)| targets.as_slice())
                            .unwrap_or(&[]);
                        broadcast_forward_events.push(lai_core::BroadcastForwardEvent {
                            protocol: "udp".to_owned(),
                            source_ip: socket_addr_ipv4(source),
                            destination_ip: socket_addr_ipv4(destination),
                            destination_port: destination.port(),
                            forwarded: !targets.is_empty(),
                            reason: if targets.is_empty() {
                                "no-forward-targets".to_owned()
                            } else {
                                "userspace-capture-forwarded".to_owned()
                            },
                            target_count: targets.len(),
                            packet_io_backend: packet_io_backend.to_owned(),
                        });
                        for target in targets {
                            let raw_virtual_packet = if forward_raw_ipv4 {
                                Some(runtime_virtual_udp_packet(
                                    plan,
                                    source,
                                    destination,
                                    &buffer[..received],
                                    broadcast,
                                ))
                            } else {
                                None
                            };
                            let raw_ipv4_packet = raw_virtual_packet
                                .as_ref()
                                .map(|packet| lai_core::build_ipv4_udp_packet(packet, 64))
                                .transpose()
                                .map_err(invalid_input)?;
                            let mut forward_payload = serde_json::json!({
                                "room_id": plan.room_id,
                                "peer_id": plan.local_peer_id,
                                "kind": "runtime-udp-forward",
                                "source": source.to_string(),
                                "destination": destination.to_string(),
                                "destination_port": destination.port(),
                                "broadcast": broadcast,
                                "bytes": STANDARD_NO_PAD.encode(&buffer[..received]),
                            });
                            if let (Some(packet), Some(raw_bytes)) =
                                (raw_virtual_packet.as_ref(), raw_ipv4_packet.as_ref())
                            {
                                forward_payload["payload_encoding"] =
                                    serde_json::json!("udp-payload+raw-ipv4");
                                forward_payload["raw_ipv4_packet"] =
                                    serde_json::json!(STANDARD_NO_PAD.encode(raw_bytes));
                                forward_payload["raw_ipv4_packet_bytes"] =
                                    serde_json::json!(raw_bytes.len());
                                forward_payload["virtual_source"] = serde_json::json!(format!(
                                    "{}:{}",
                                    packet.source_ip, packet.source_port
                                ));
                                forward_payload["virtual_destination"] =
                                    serde_json::json!(format!(
                                        "{}:{}",
                                        packet.destination_ip, packet.destination_port
                                    ));
                            }
                            let envelope = seal_tunnel_payload(
                                key,
                                "runtime-udp-forward",
                                forwarded_packets.len() as u64 + 1,
                                current_epoch_ms(),
                                serde_json::to_string(&forward_payload)?.as_bytes(),
                            )?;
                            let wire = serde_json::to_vec(&envelope)?;
                            match tunnel_socket.send_to(&wire, target) {
                                Ok(sent) => {
                                    bytes_sent += sent as u64;
                                    forwarded_packets.push(serde_json::json!({
                                        "target": target.to_string(),
                                        "source": source.to_string(),
                                        "destination": destination.to_string(),
                                        "bytesSent": sent,
                                        "payloadBytes": received,
                                        "rawIpv4PacketBytes": raw_ipv4_packet.as_ref().map(Vec::len),
                                        "sentAtMs": current_epoch_ms(),
                                    }));
                                }
                                Err(err) => {
                                    last_error = Some(format!(
                                        "Failed to forward UDP packet to {target}: {err}"
                                    ));
                                }
                            }
                        }
                    }
                }
                Err(err)
                    if matches!(
                        err.kind(),
                        ErrorKind::WouldBlock
                            | ErrorKind::TimedOut
                            | ErrorKind::Interrupted
                            | ErrorKind::ConnectionReset
                    ) => {}
                Err(err) => return Err(err.into()),
            }
        }

        if let Some(session) = wintun_packet_io.as_mut() {
            match session.receive_once() {
                Ok(Some(packet)) => {
                    let packet_index = wintun_runtime_received_packets.len() + 1;
                    match (&packet.parsed_udp, &packet.parsed_tcp, &packet.summary) {
                        (Some(udp_packet), _, _) => {
                            let observation =
                                lai_core::udp_observation_from_virtual_packet(udp_packet);
                            observation_lines.push(
                                lai_core::packet_observation_line_from_udp_forward(&observation),
                            );
                            append_observation_lines(
                                observe_file,
                                std::slice::from_ref(&observation),
                            )?;
                            capture_summaries.push(PacketCaptureSummary {
                                protocol: "udp".to_owned(),
                                source_ip: udp_packet.source_ip,
                                destination_ip: udp_packet.destination_ip,
                                destination_port: udp_packet.destination_port,
                                direction: "virtual-adapter".to_owned(),
                                broadcast: udp_packet.broadcast,
                                packet_count: 1,
                                bytes: udp_packet.payload.len() as u32,
                            });
                            let (broadcast_decision, should_forward_udp) = if udp_packet.broadcast {
                                let packet = lai_core::BroadcastPacket {
                                    protocol: "udp".to_owned(),
                                    source_ip: udp_packet.source_ip,
                                    destination_ip: udp_packet.destination_ip,
                                    destination_port: udp_packet.destination_port,
                                };
                                let decision = broadcast_gate.decide(&packet, current_epoch_ms());
                                let should_forward =
                                    decision.forward && !heartbeat_targets.is_empty();
                                broadcast_forward_events.push(lai_core::BroadcastForwardEvent {
                                    protocol: "udp".to_owned(),
                                    source_ip: udp_packet.source_ip,
                                    destination_ip: udp_packet.destination_ip,
                                    destination_port: udp_packet.destination_port,
                                    forwarded: should_forward,
                                    reason: if decision.forward && heartbeat_targets.is_empty() {
                                        "no-forward-targets".to_owned()
                                    } else {
                                        decision.reason.clone()
                                    },
                                    target_count: if should_forward {
                                        heartbeat_targets.len()
                                    } else {
                                        0
                                    },
                                    packet_io_backend: packet_io_backend.to_owned(),
                                });
                                (Some(decision), should_forward)
                            } else {
                                (None, !heartbeat_targets.is_empty())
                            };
                            wintun_runtime_received_packets.push(serde_json::json!({
                                "packetIndex": packet_index,
                                "packetBytes": packet.packet_bytes,
                                "sourceIp": udp_packet.source_ip,
                                "destinationIp": udp_packet.destination_ip,
                                "sourcePort": udp_packet.source_port,
                                "destinationPort": udp_packet.destination_port,
                                "payloadBytes": udp_packet.payload.len(),
                                "broadcast": udp_packet.broadcast,
                                "forwarded": should_forward_udp,
                                "broadcastDecision": broadcast_decision,
                            }));

                            let udp_forward_targets = if should_forward_udp {
                                heartbeat_targets.as_slice()
                            } else {
                                &[]
                            };
                            for target in udp_forward_targets {
                                let forward_payload = serde_json::json!({
                                    "room_id": plan.room_id,
                                    "peer_id": plan.local_peer_id,
                                    "kind": "runtime-udp-forward",
                                    "source": format!("{}:{}", udp_packet.source_ip, udp_packet.source_port),
                                    "destination": format!("{}:{}", udp_packet.destination_ip, udp_packet.destination_port),
                                    "destination_port": udp_packet.destination_port,
                                    "broadcast": udp_packet.broadcast,
                                    "bytes": STANDARD_NO_PAD.encode(&udp_packet.payload),
                                    "payload_encoding": "udp-payload+raw-ipv4",
                                    "raw_ipv4_packet": STANDARD_NO_PAD.encode(&packet.bytes),
                                    "raw_ipv4_packet_bytes": packet.packet_bytes,
                                    "virtual_source": format!("{}:{}", udp_packet.source_ip, udp_packet.source_port),
                                    "virtual_destination": format!("{}:{}", udp_packet.destination_ip, udp_packet.destination_port),
                                });
                                let envelope = seal_tunnel_payload(
                                    key,
                                    "runtime-udp-forward",
                                    forwarded_packets.len() as u64 + 1,
                                    current_epoch_ms(),
                                    serde_json::to_string(&forward_payload)?.as_bytes(),
                                )?;
                                let wire = serde_json::to_vec(&envelope)?;
                                match tunnel_socket.send_to(&wire, target) {
                                    Ok(sent) => {
                                        bytes_sent += sent as u64;
                                        forwarded_packets.push(serde_json::json!({
                                            "target": target.to_string(),
                                            "source": format!("{}:{}", udp_packet.source_ip, udp_packet.source_port),
                                            "destination": format!("{}:{}", udp_packet.destination_ip, udp_packet.destination_port),
                                            "bytesSent": sent,
                                            "payloadBytes": udp_packet.payload.len(),
                                            "rawIpv4PacketBytes": packet.packet_bytes,
                                            "packetIoBackend": "wintun",
                                            "sentAtMs": current_epoch_ms(),
                                        }));
                                    }
                                    Err(err) => {
                                        last_error = Some(format!(
                                            "Failed to forward Wintun packet to {target}: {err}"
                                        ));
                                    }
                                }
                            }
                        }
                        (_, Some(tcp_packet), Some(summary)) => {
                            let observation =
                                lai_core::tcp_observation_from_virtual_packet(tcp_packet);
                            let line = lai_core::packet_observation_line_from_transport(
                                "tcp",
                                &observation,
                            );
                            observation_lines.push(line.clone());
                            append_observation_text_lines(observe_file, &[line])?;
                            capture_summaries.push(PacketCaptureSummary {
                                protocol: "tcp".to_owned(),
                                source_ip: tcp_packet.source_ip,
                                destination_ip: tcp_packet.destination_ip,
                                destination_port: tcp_packet.destination_port,
                                direction: "virtual-adapter".to_owned(),
                                broadcast: false,
                                packet_count: 1,
                                bytes: tcp_packet.payload.len() as u32,
                            });
                            wintun_runtime_received_packets.push(serde_json::json!({
                                "packetIndex": packet_index,
                                "protocol": "tcp",
                                "packetBytes": packet.packet_bytes,
                                "sourceIp": tcp_packet.source_ip,
                                "destinationIp": tcp_packet.destination_ip,
                                "sourcePort": tcp_packet.source_port,
                                "destinationPort": tcp_packet.destination_port,
                                "payloadBytes": tcp_packet.payload.len(),
                                "flags": tcp_packet.flags,
                                "forwarded": !heartbeat_targets.is_empty(),
                            }));

                            for target in &heartbeat_targets {
                                let forward_payload = serde_json::json!({
                                    "room_id": plan.room_id,
                                    "peer_id": plan.local_peer_id,
                                    "kind": "runtime-ipv4-forward",
                                    "source": format!("{}:{}", tcp_packet.source_ip, tcp_packet.source_port),
                                    "destination": format!("{}:{}", tcp_packet.destination_ip, tcp_packet.destination_port),
                                    "destination_port": tcp_packet.destination_port,
                                    "broadcast": false,
                                    "payload_encoding": "raw-ipv4",
                                    "raw_ipv4_packet": STANDARD_NO_PAD.encode(&packet.bytes),
                                    "raw_ipv4_packet_bytes": packet.packet_bytes,
                                    "virtual_source": format!("{}:{}", tcp_packet.source_ip, tcp_packet.source_port),
                                    "virtual_destination": format!("{}:{}", tcp_packet.destination_ip, tcp_packet.destination_port),
                                    "ipv4_protocol": summary.protocol.clone(),
                                    "ipv4_protocol_number": summary.protocol_number,
                                });
                                let envelope = seal_tunnel_payload(
                                    key,
                                    "runtime-ipv4-forward",
                                    forwarded_packets.len() as u64 + 1,
                                    current_epoch_ms(),
                                    serde_json::to_string(&forward_payload)?.as_bytes(),
                                )?;
                                let wire = serde_json::to_vec(&envelope)?;
                                match tunnel_socket.send_to(&wire, target) {
                                    Ok(sent) => {
                                        bytes_sent += sent as u64;
                                        forwarded_packets.push(serde_json::json!({
                                            "target": target.to_string(),
                                            "source": format!("{}:{}", tcp_packet.source_ip, tcp_packet.source_port),
                                            "destination": format!("{}:{}", tcp_packet.destination_ip, tcp_packet.destination_port),
                                            "bytesSent": sent,
                                            "payloadBytes": tcp_packet.payload.len(),
                                            "rawIpv4PacketBytes": packet.packet_bytes,
                                            "packetIoBackend": "wintun",
                                            "protocol": "tcp",
                                            "sentAtMs": current_epoch_ms(),
                                        }));
                                    }
                                    Err(err) => {
                                        last_error = Some(format!(
                                            "Failed to forward Wintun TCP packet to {target}: {err}"
                                        ));
                                    }
                                }
                            }
                        }
                        (_, _, Some(summary)) => {
                            wintun_runtime_received_packets.push(serde_json::json!({
                                "packetIndex": packet_index,
                                "protocol": summary.protocol.clone(),
                                "protocolNumber": summary.protocol_number,
                                "packetBytes": packet.packet_bytes,
                                "sourceIp": summary.source_ip,
                                "destinationIp": summary.destination_ip,
                                "payloadBytes": summary.payload_bytes,
                                "forwarded": !heartbeat_targets.is_empty(),
                            }));

                            for target in &heartbeat_targets {
                                let forward_payload = serde_json::json!({
                                    "room_id": plan.room_id,
                                    "peer_id": plan.local_peer_id,
                                    "kind": "runtime-ipv4-forward",
                                    "source": summary.source_ip.to_string(),
                                    "destination": summary.destination_ip.to_string(),
                                    "broadcast": summary.broadcast,
                                    "payload_encoding": "raw-ipv4",
                                    "raw_ipv4_packet": STANDARD_NO_PAD.encode(&packet.bytes),
                                    "raw_ipv4_packet_bytes": packet.packet_bytes,
                                    "ipv4_protocol": summary.protocol.clone(),
                                    "ipv4_protocol_number": summary.protocol_number,
                                });
                                let envelope = seal_tunnel_payload(
                                    key,
                                    "runtime-ipv4-forward",
                                    forwarded_packets.len() as u64 + 1,
                                    current_epoch_ms(),
                                    serde_json::to_string(&forward_payload)?.as_bytes(),
                                )?;
                                let wire = serde_json::to_vec(&envelope)?;
                                match tunnel_socket.send_to(&wire, target) {
                                    Ok(sent) => {
                                        bytes_sent += sent as u64;
                                        forwarded_packets.push(serde_json::json!({
                                            "target": target.to_string(),
                                            "source": summary.source_ip,
                                            "destination": summary.destination_ip,
                                            "bytesSent": sent,
                                            "payloadBytes": summary.payload_bytes,
                                            "rawIpv4PacketBytes": packet.packet_bytes,
                                            "packetIoBackend": "wintun",
                                            "protocol": summary.protocol.clone(),
                                            "sentAtMs": current_epoch_ms(),
                                        }));
                                    }
                                    Err(err) => {
                                        last_error = Some(format!(
                                            "Failed to forward Wintun IPv4 packet to {target}: {err}"
                                        ));
                                    }
                                }
                            }
                        }
                        (_, _, None) => {
                            wintun_runtime_received_packets.push(serde_json::json!({
                                "packetIndex": packet_index,
                                "packetBytes": packet.packet_bytes,
                                "forwarded": false,
                                "parseError": packet.parse_error.clone(),
                            }));
                        }
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    let message = format!("Failed to read raw IPv4 packet from Wintun: {err}");
                    last_error = Some(message.clone());
                    wintun_runtime_errors.push(message);
                }
            }
        }

        if let Some(receiver) = &inject_receiver {
            match receiver.recv_from(&mut buffer) {
                Ok((received, source)) => {
                    injected_received_packets.push(serde_json::json!({
                        "source": source.to_string(),
                        "target": receiver.local_addr()?.to_string(),
                        "bytesReceived": received,
                    }));
                }
                Err(err)
                    if matches!(
                        err.kind(),
                        ErrorKind::WouldBlock
                            | ErrorKind::TimedOut
                            | ErrorKind::Interrupted
                            | ErrorKind::ConnectionReset
                    ) => {}
                Err(err) => return Err(err.into()),
            }
        }
    }

    if last_error.is_none() && peer_timed_out {
        last_error = Some("Runtime tunnel peer timed out before the runtime stopped.".to_owned());
    }

    let tunnel_snapshot = TunnelServiceSnapshot {
        service_running: true,
        connected_peer_count,
        connection_path: if connected_peer_count > 0 {
            Some("p2p".to_owned())
        } else {
            None
        },
        average_latency_ms: Some(duration_ms.min(u32::MAX as u64) as u32),
        packet_loss_percent: if self_probe && connected_peer_count == 0 {
            Some(100.0)
        } else {
            Some(0.0)
        },
        bytes_sent,
        bytes_received,
        last_error,
    };
    let heartbeat_packets_sent = heartbeat_packets.len();
    let broadcast_forward_report = lai_core::create_broadcast_forward_report(
        broadcast_gate.policy(),
        broadcast_forward_events,
    );
    let wintun_runtime_close = wintun_packet_io
        .as_mut()
        .map(|session| serde_json::to_value(session.close()))
        .transpose()?
        .unwrap_or_else(|| {
            serde_json::json!({
                "session_ended": false,
                "closed": false,
            })
        });
    let tunnel_endpoint_text = tunnel_endpoint.to_string();
    let runtime_peer_summary_values = runtime_peer_summaries(
        plan,
        &[],
        &tunnel_packets,
        &forwarded_packets,
        &heartbeat_packets,
        &heartbeat_ack_packets,
        if connected_peer_count > 0 {
            Some("p2p")
        } else {
            None
        },
        Some(tunnel_endpoint_text.as_str()),
    );
    let runtime_peer_observations =
        runtime_peer_observations_from_summaries(&runtime_peer_summary_values);
    let expected_peer_count = if runtime_peer_observations.is_empty() {
        if plan.peers.is_empty() {
            connected_peer_count
        } else {
            plan.peers.len() as u16
        }
    } else {
        runtime_peer_observations.len() as u16
    };
    let network_report =
        evaluate_network_observations(lai_core::network_snapshot_from_runtime_with_peers(
            None,
            Some(tunnel_snapshot.clone()),
            &capture_summaries,
            runtime_peer_observations,
            expected_peer_count,
            runtime_expected_broadcast_ports,
            runtime_expected_game_ports,
        ));
    let result = serde_json::json!({
        "status": if tunnel_snapshot.last_error.is_none() { "ok" } else { "degraded" },
        "startedAtMs": started_at_ms,
        "durationMs": duration_ms,
        "stopReason": stop_reason,
        "plan": plan,
        "packetIoPlan": packet_io_plan,
        "packetIoProbe": packet_io_probe.clone(),
        "adapterWriteStatus": packet_io_probe["adapterWriteStatus"].clone(),
        "adapterReadStatus": packet_io_probe["adapterReadStatus"].clone(),
        "forwardRawIpv4": forward_raw_ipv4,
        "actualTunnelEndpoint": tunnel_endpoint_text,
        "tunnelServiceSnapshot": tunnel_snapshot,
        "heartbeatTargets": heartbeat_targets.iter().map(SocketAddr::to_string).collect::<Vec<_>>(),
        "heartbeatPackets": heartbeat_packets,
        "heartbeatPacketsSent": heartbeat_packets_sent,
        "heartbeatAckPackets": heartbeat_ack_packets,
        "coordinationMonitorReports": coordination_monitor_reports,
        "snapshotWriteCount": snapshot_write_count,
        "tunnelPackets": tunnel_packets,
        "forwardedPackets": forwarded_packets,
        "broadcastForwardReport": broadcast_forward_report,
        "rawVirtualPackets": raw_virtual_packets,
        "wintunRuntime": {
            "enabled": wintun_runtime,
            "open": wintun_runtime_open,
            "close": wintun_runtime_close,
            "receivedPackets": wintun_runtime_received_packets,
            "sentPackets": wintun_runtime_sent_packets,
            "errors": wintun_runtime_errors,
        },
        "runtimeCleanupPlan": runtime_cleanup_plan,
        "injectedPackets": injected_packets,
        "injectedReceivedPackets": injected_received_packets,
        "injectTarget": inject_target.map(|target| target.to_string()),
        "packetCaptureSummaries": capture_summaries,
        "packetObservationLines": observation_lines,
        "networkObservation": network_report,
        "runtimePeerSummaries": runtime_peer_summary_values,
    });
    if let Some(path) = snapshot_out {
        write_json_file(path, &result)?;
    }
    Ok(result)
}

fn runtime_packet_io_probe(
    packet_io_backend: &str,
    options: &RuntimePacketIoProbeOptions,
) -> serde_json::Value {
    match packet_io_backend {
        "userspace-udp" => serde_json::json!({
            "backend": "userspace-udp",
            "status": "ready",
            "adapterReadStatus": "not-required",
            "adapterWriteStatus": "not-required",
            "detail": "Using user-space UDP sockets; no virtual adapter packet session is required.",
        }),
        "wintun" => {
            let session_probe =
                lai_core::probe_wintun_session(lai_core::WintunSessionProbeRequest {
                    adapter_name: options.wintun_adapter_name.clone(),
                    tunnel_type: "LocalAreaInterconnection".to_owned(),
                    ring_capacity: options.wintun_ring_capacity,
                });
            let receive_probe = options.wintun_probe_receive.then(|| {
                lai_core::probe_wintun_packet_receive(lai_core::WintunPacketReceiveProbeRequest {
                    adapter_name: options.wintun_adapter_name.clone(),
                    ring_capacity: options.wintun_ring_capacity,
                    max_attempts: options.wintun_receive_attempts,
                    poll_interval_ms: options.wintun_receive_poll_interval_ms,
                })
            });
            let send_probe = options.wintun_probe_send.then(|| {
                lai_core::probe_wintun_packet_send(lai_core::WintunPacketSendProbeRequest {
                    adapter_name: options.wintun_adapter_name.clone(),
                    ring_capacity: options.wintun_ring_capacity,
                    packet: VirtualUdpPacket {
                        source_ip: Ipv4Addr::new(10, 77, 12, 2),
                        destination_ip: Ipv4Addr::new(10, 77, 12, 255),
                        source_port: 39077,
                        destination_port: 27015,
                        payload: b"runtime-wintun-send-probe".to_vec(),
                        broadcast: true,
                    },
                })
            });

            let session_ready = session_probe.status == "session-started-and-ended";
            let receive_status = receive_probe
                .as_ref()
                .map(|probe| probe.status.as_str())
                .unwrap_or("not-run");
            let send_status = send_probe
                .as_ref()
                .map(|probe| probe.status.as_str())
                .unwrap_or("not-run-needs-confirmation");
            let read_ready = matches!(receive_status, "empty" | "packet-received");
            let write_ready = send_status == "packet-sent";
            let write_deferred = send_status == "not-run-needs-confirmation";
            let adapter_read_status = if read_ready {
                "ready"
            } else if receive_status == "not-run" {
                "not-run"
            } else {
                "unavailable"
            };
            let adapter_write_status = if write_ready {
                "ready"
            } else if write_deferred {
                "not-run-needs-confirmation"
            } else {
                "unavailable"
            };
            let status = if session_ready && read_ready && write_ready {
                "ready"
            } else if session_ready && read_ready && write_deferred {
                "partial"
            } else {
                "unavailable"
            };

            serde_json::json!({
                "backend": "wintun",
                "status": status,
                "adapterName": options.wintun_adapter_name,
                "ringCapacity": options.wintun_ring_capacity,
                "adapterReadStatus": adapter_read_status,
                "adapterWriteStatus": adapter_write_status,
                "sessionProbe": session_probe,
                "receiveProbe": receive_probe.unwrap_or_else(|| lai_core::WintunPacketReceiveProbeReport {
                    status: "not-run".to_owned(),
                    adapter_name: Some(options.wintun_adapter_name.clone()),
                    ring_capacity: options.wintun_ring_capacity,
                    max_attempts: options.wintun_receive_attempts,
                    poll_interval_ms: options.wintun_receive_poll_interval_ms,
                    opened: false,
                    session_started: false,
                    receive_attempts: 0,
                    packet_received: false,
                    packet_released: false,
                    session_ended: false,
                    closed: false,
                    packet: None,
                    error: Some("Wintun receive probe disabled for this runtime run.".to_owned()),
                }),
                "sendProbe": send_probe.map(serde_json::to_value).transpose().expect("serialize Wintun send probe").unwrap_or_else(|| serde_json::json!({
                    "status": "not-run-needs-confirmation",
                    "adapter_name": options.wintun_adapter_name,
                    "ring_capacity": options.wintun_ring_capacity,
                    "packet_sent": false,
                    "next_action": "Pass --wintun-probe-send true from an administrator terminal to execute one controlled Wintun send probe."
                })),
            })
        }
        "tap" => serde_json::json!({
            "backend": "tap",
            "status": "unavailable",
            "adapterReadStatus": "unavailable",
            "adapterWriteStatus": "unavailable",
            "detail": "TAP packet I/O is planned but not implemented yet.",
        }),
        other => serde_json::json!({
            "backend": other,
            "status": "unknown-backend",
            "adapterReadStatus": "unavailable",
            "adapterWriteStatus": "unavailable",
            "detail": "Unknown packet I/O backend.",
        }),
    }
}

fn run_udp_forwarder(
    listen: &str,
    forward: &str,
    max_packets: u16,
    timeout_ms: u64,
    observe_file: Option<&str>,
    broadcast: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind(listen)?;
    socket.set_broadcast(true)?;
    socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let forward = forward.parse::<SocketAddr>()?;
    let mut buffer = vec![0u8; 65_535];
    let mut observations = Vec::new();

    for _ in 0..max_packets {
        match socket.recv_from(&mut buffer) {
            Ok((received, source)) => {
                socket.send_to(&buffer[..received], forward)?;
                observations.push(UdpForwardObservation {
                    source,
                    destination: forward,
                    bytes: received,
                    broadcast,
                    direction: "outbound".to_owned(),
                });
            }
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::WouldBlock
                        | ErrorKind::TimedOut
                        | ErrorKind::Interrupted
                        | ErrorKind::ConnectionReset
                ) =>
            {
                break;
            }
            Err(err) => return Err(err.into()),
        }
    }

    let summary = udp_forward_summary(&observations);
    append_observation_lines(observe_file, &observations)?;
    Ok(serde_json::json!({
        "status": if summary.forwarded_packets == 0 { "timeout" } else { "ok" },
        "listen": socket.local_addr()?.to_string(),
        "forward": forward.to_string(),
        "summary": summary,
    }))
}

fn run_udp_capture(
    listen: &str,
    max_packets: u16,
    timeout_ms: u64,
    observe_file: Option<&str>,
    broadcast: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind(listen)?;
    socket.set_broadcast(true)?;
    socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let destination = socket.local_addr()?;
    let mut buffer = vec![0u8; 65_535];
    let mut observations = Vec::new();

    for _ in 0..max_packets {
        match socket.recv_from(&mut buffer) {
            Ok((received, source)) => {
                observations.push(UdpForwardObservation {
                    source,
                    destination,
                    bytes: received,
                    broadcast,
                    direction: "inbound".to_owned(),
                });
            }
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::WouldBlock
                        | ErrorKind::TimedOut
                        | ErrorKind::Interrupted
                        | ErrorKind::ConnectionReset
                ) =>
            {
                break;
            }
            Err(err) => return Err(err.into()),
        }
    }

    let summary = udp_forward_summary(&observations);
    append_observation_lines(observe_file, &observations)?;
    Ok(serde_json::json!({
        "status": if summary.forwarded_packets == 0 { "timeout" } else { "ok" },
        "listen": destination.to_string(),
        "summary": summary,
    }))
}

fn run_udp_forward_loopback_test(
    message: &str,
    observe_file: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let target = UdpSocket::bind("127.0.0.1:0")?;
    target.set_read_timeout(Some(Duration::from_millis(2000)))?;
    let forwarder = UdpSocket::bind("127.0.0.1:0")?;
    forwarder.set_read_timeout(Some(Duration::from_millis(2000)))?;
    let client = UdpSocket::bind("127.0.0.1:0")?;
    client.send_to(message.as_bytes(), forwarder.local_addr()?)?;

    let mut buffer = vec![0u8; 65_535];
    let (received, source) = forwarder.recv_from(&mut buffer)?;
    forwarder.send_to(&buffer[..received], target.local_addr()?)?;
    let (target_received, _) = target.recv_from(&mut buffer)?;

    let observation = UdpForwardObservation {
        source,
        destination: target.local_addr()?,
        bytes: received,
        broadcast: false,
        direction: "outbound".to_owned(),
    };
    append_observation_lines(observe_file, std::slice::from_ref(&observation))?;
    let summary = udp_forward_summary(&[observation]);

    Ok(serde_json::json!({
        "status": "ok",
        "client": client.local_addr()?.to_string(),
        "forwarder": forwarder.local_addr()?.to_string(),
        "target": target.local_addr()?.to_string(),
        "message": String::from_utf8_lossy(&buffer[..target_received]),
        "summary": summary,
    }))
}

fn run_udp_capture_loopback_test(
    message: &str,
    observe_file: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let capture = UdpSocket::bind("127.0.0.1:0")?;
    capture.set_read_timeout(Some(Duration::from_millis(2000)))?;
    let client = UdpSocket::bind("127.0.0.1:0")?;
    client.send_to(message.as_bytes(), capture.local_addr()?)?;

    let mut buffer = vec![0u8; 65_535];
    let (received, source) = capture.recv_from(&mut buffer)?;
    let observation = UdpForwardObservation {
        source,
        destination: capture.local_addr()?,
        bytes: received,
        broadcast: false,
        direction: "inbound".to_owned(),
    };
    append_observation_lines(observe_file, std::slice::from_ref(&observation))?;
    let summary = udp_forward_summary(&[observation]);

    Ok(serde_json::json!({
        "status": "ok",
        "client": client.local_addr()?.to_string(),
        "capture": capture.local_addr()?.to_string(),
        "message": String::from_utf8_lossy(&buffer[..received]),
        "summary": summary,
    }))
}

fn run_udp_loopback_test(
    port: u16,
    message: &str,
    timeout_ms: u64,
    observe_file: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let listener = UdpSocket::bind(("127.0.0.1", port))?;
    listener.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let sender = UdpSocket::bind("127.0.0.1:0")?;
    let start = Instant::now();
    sender.send_to(message.as_bytes(), listener.local_addr()?)?;

    let mut buffer = vec![0u8; 65_535];
    let (received, source) = listener.recv_from(&mut buffer)?;
    let received_message = String::from_utf8_lossy(&buffer[..received]).to_string();
    let observation = UdpForwardObservation {
        source,
        destination: listener.local_addr()?,
        bytes: received,
        broadcast: false,
        direction: "inbound".to_owned(),
    };
    append_observation_lines(observe_file, std::slice::from_ref(&observation))?;

    Ok(serde_json::json!({
        "status": if received_message == message { "ok" } else { "mismatch" },
        "protocol": "udp",
        "localAddress": "127.0.0.1",
        "port": port,
        "bytesReceived": received,
        "elapsedMs": start.elapsed().as_millis() as u64,
        "message": received_message,
        "packetObservationLine": lai_core::packet_observation_line_from_udp_forward(&observation),
    }))
}

fn run_udp_broadcast_test(
    port: u16,
    message: &str,
    timeout_ms: u64,
    observe_file: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let listener = UdpSocket::bind(("0.0.0.0", port))?;
    listener.set_broadcast(true)?;
    listener.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let sender = UdpSocket::bind("0.0.0.0:0")?;
    sender.set_broadcast(true)?;
    let broadcast_destination = format!("255.255.255.255:{port}").parse::<SocketAddr>()?;
    let start = Instant::now();
    sender.send_to(message.as_bytes(), broadcast_destination)?;

    let mut buffer = vec![0u8; 65_535];
    let (received, source) = listener.recv_from(&mut buffer)?;
    let received_message = String::from_utf8_lossy(&buffer[..received]).to_string();
    let observation = UdpForwardObservation {
        source,
        destination: broadcast_destination,
        bytes: received,
        broadcast: true,
        direction: "inbound".to_owned(),
    };
    append_observation_lines(observe_file, std::slice::from_ref(&observation))?;

    Ok(serde_json::json!({
        "status": if received_message == message { "ok" } else { "mismatch" },
        "protocol": "udp",
        "broadcastAddress": "255.255.255.255",
        "port": port,
        "remote": source.to_string(),
        "bytesReceived": received,
        "elapsedMs": start.elapsed().as_millis() as u64,
        "message": received_message,
        "packetObservationLine": lai_core::packet_observation_line_from_udp_forward(&observation),
    }))
}

fn run_tcp_loopback_test(
    port: u16,
    message: &str,
    timeout_ms: u64,
    observe_file: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let start = Instant::now();
    let mut client = TcpStream::connect(("127.0.0.1", port))?;
    client.set_write_timeout(Some(Duration::from_millis(timeout_ms)))?;
    client.write_all(message.as_bytes())?;
    client.shutdown(std::net::Shutdown::Write)?;

    let (mut accepted, source) = listener.accept()?;
    accepted.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
    let mut buffer = vec![0u8; 4096];
    let received = accepted.read(&mut buffer)?;
    let received_message = String::from_utf8_lossy(&buffer[..received]).to_string();
    let destination = listener.local_addr()?;
    let observation = UdpForwardObservation {
        source,
        destination,
        bytes: received,
        broadcast: false,
        direction: "inbound".to_owned(),
    };
    let observation_line = lai_core::packet_observation_line_from_transport("tcp", &observation);
    append_observation_text_lines(observe_file, std::slice::from_ref(&observation_line))?;

    Ok(serde_json::json!({
        "status": if received_message == message { "ok" } else { "mismatch" },
        "protocol": "tcp",
        "localAddress": "127.0.0.1",
        "port": port,
        "bytesReceived": received,
        "elapsedMs": start.elapsed().as_millis() as u64,
        "message": received_message,
        "packetObservationLine": observation_line,
    }))
}

fn append_observation_lines(
    observe_file: Option<&str>,
    observations: &[UdpForwardObservation],
) -> Result<(), Box<dyn std::error::Error>> {
    let lines = observations
        .iter()
        .map(lai_core::packet_observation_line_from_udp_forward)
        .collect::<Vec<_>>();
    append_observation_text_lines(observe_file, &lines)
}

fn append_observation_text_lines(
    observe_file: Option<&str>,
    lines: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = observe_file else {
        return Ok(());
    };
    if lines.is_empty() {
        return Ok(());
    }
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for line in lines {
        writeln!(file, "{line}")?;
    }
    Ok(())
}

fn random_nonce() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    STANDARD_NO_PAD.encode(bytes)
}

fn normalize_runtime_expected_ports(
    mut expected_ports: Vec<u16>,
    actual_ports: &[u16],
) -> Vec<u16> {
    if expected_ports.contains(&0) {
        expected_ports.retain(|port| *port != 0);
        expected_ports.extend(actual_ports.iter().copied());
        expected_ports.sort_unstable();
        expected_ports.dedup();
    }
    expected_ports
}

fn runtime_subnet_from_local_ip(local_ip: Ipv4Addr) -> Ipv4Subnet {
    let octets = local_ip.octets();
    Ipv4Subnet {
        network: Ipv4Addr::new(octets[0], octets[1], octets[2], 0),
        prefix: 24,
    }
}

fn runtime_forward_targets(
    plan: &RoomRuntimePlan,
    actual_broadcast_ports: &[u16],
    forward_self_probe: bool,
    tunnel_endpoint: SocketAddr,
) -> Result<Vec<(u16, Vec<SocketAddr>)>, Box<dyn std::error::Error>> {
    let mut targets_by_port = Vec::new();
    for forwarder in &plan.udp_forwarders {
        let forward_port = if forwarder.port == 0 {
            actual_broadcast_ports.first().copied().unwrap_or(0)
        } else {
            forwarder.port
        };
        if forward_port == 0 {
            continue;
        }
        let mut targets = forwarder
            .forward_to_peers
            .iter()
            .map(|endpoint| {
                endpoint.parse::<SocketAddr>().map_err(|err| {
                    invalid_input(format!(
                        "invalid runtime forward endpoint `{endpoint}`: {err}"
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if forward_self_probe {
            targets.push(tunnel_endpoint);
        }
        targets_by_port.push((forward_port, targets));
    }
    if forward_self_probe && targets_by_port.is_empty() {
        targets_by_port.extend(
            actual_broadcast_ports
                .iter()
                .copied()
                .map(|port| (port, vec![tunnel_endpoint])),
        );
    }
    Ok(targets_by_port)
}

fn runtime_heartbeat_targets(
    plan: &RoomRuntimePlan,
    self_probe: bool,
    tunnel_endpoint: SocketAddr,
) -> Result<Vec<SocketAddr>, Box<dyn std::error::Error>> {
    let mut targets = plan
        .peers
        .iter()
        .map(|peer| {
            peer.endpoint.parse::<SocketAddr>().map_err(|err| {
                invalid_input(format!(
                    "invalid runtime heartbeat endpoint `{}`: {err}",
                    peer.endpoint
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if self_probe {
        targets.push(tunnel_endpoint);
    }
    targets.sort_unstable();
    targets.dedup();
    Ok(targets)
}

fn refresh_runtime_network_observation(
    result: &mut serde_json::Value,
    plan: &RoomRuntimePlan,
    runtime_peer_summary_values: Vec<serde_json::Value>,
    expected_broadcast_ports: Vec<u16>,
    expected_game_ports: Vec<u16>,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime_peer_observations =
        runtime_peer_observations_from_summaries(&runtime_peer_summary_values);
    let connected_peer_count = result
        .get("tunnelServiceSnapshot")
        .and_then(|snapshot| snapshot.get("connected_peer_count"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default()
        .min(u16::MAX as u64) as u16;
    let expected_peer_count = if runtime_peer_observations.is_empty() {
        if plan.peers.is_empty() {
            connected_peer_count
        } else {
            plan.peers.len().min(u16::MAX as usize) as u16
        }
    } else {
        runtime_peer_observations.len().min(u16::MAX as usize) as u16
    };
    let tunnel_snapshot = result
        .get("tunnelServiceSnapshot")
        .cloned()
        .map(serde_json::from_value::<TunnelServiceSnapshot>)
        .transpose()?;
    let capture_summaries = json_array_values(result, "packetCaptureSummaries")
        .into_iter()
        .map(serde_json::from_value::<PacketCaptureSummary>)
        .collect::<Result<Vec<_>, _>>()?;
    let actual_broadcast_ports = capture_summaries
        .iter()
        .filter(|summary| summary.broadcast)
        .map(|summary| summary.destination_port)
        .collect::<Vec<_>>();
    let actual_game_ports = capture_summaries
        .iter()
        .filter(|summary| !summary.broadcast)
        .map(|summary| summary.destination_port)
        .collect::<Vec<_>>();
    let expected_broadcast_ports =
        normalize_runtime_expected_ports(expected_broadcast_ports, &actual_broadcast_ports);
    let expected_game_ports =
        normalize_runtime_expected_ports(expected_game_ports, &actual_game_ports);
    let network_report =
        evaluate_network_observations(lai_core::network_snapshot_from_runtime_with_peers(
            None,
            tunnel_snapshot,
            &capture_summaries,
            runtime_peer_observations,
            expected_peer_count,
            expected_broadcast_ports,
            expected_game_ports,
        ));
    result["networkObservation"] = serde_json::to_value(network_report)?;
    result["runtimePeerSummaries"] = serde_json::Value::Array(runtime_peer_summary_values);
    Ok(())
}

fn runtime_peer_summaries(
    plan: &RoomRuntimePlan,
    connection_path_reports: &[serde_json::Value],
    tunnel_packets: &[serde_json::Value],
    forwarded_packets: &[serde_json::Value],
    heartbeat_packets: &[serde_json::Value],
    heartbeat_ack_packets: &[serde_json::Value],
    runtime_path: Option<&str>,
    local_endpoint: Option<&str>,
) -> Vec<serde_json::Value> {
    runtime_observed_peers(
        plan,
        tunnel_packets,
        heartbeat_packets,
        heartbeat_ack_packets,
        local_endpoint,
    )
    .iter()
    .map(|peer| {
        let path_entry = connection_path_reports
            .iter()
            .find(|entry| connection_path_peer_id(entry).as_deref() == Some(&peer.peer_id));
        let report = path_entry
            .and_then(|entry| entry.get("report"))
            .or(path_entry);
        let selected_path = report
            .and_then(|report| report.get("selected_path"))
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                runtime_path.filter(|_| peer_has_tunnel_packets(tunnel_packets, &peer.endpoint))
            })
            .unwrap_or("unknown");
        let connection_path_status = report
            .and_then(|report| report.get("status"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| {
                if peer_has_tunnel_packets(tunnel_packets, &peer.endpoint) {
                    "observed"
                } else {
                    "unknown"
                }
            });
        let bootstrap_status = path_entry
            .and_then(|entry| entry.get("bootstrapStatus"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("not-run");
        let latency_ms = path_entry
            .and_then(|entry| entry.get("bootstrapLatencyMs"))
            .and_then(serde_json::Value::as_u64)
            .or_else(|| max_json_u64(heartbeat_ack_packets, "peer", &peer.endpoint, "roundTripMs"));
        let bytes_sent = sum_json_bytes(heartbeat_packets, "target", &peer.endpoint, "bytesSent")
            + sum_json_bytes(heartbeat_ack_packets, "target", &peer.endpoint, "bytesSent")
            + sum_json_bytes(forwarded_packets, "target", &peer.endpoint, "bytesSent");
        let bytes_received = sum_json_bytes(tunnel_packets, "peer", &peer.endpoint, "bytes");
        let heartbeat_packets_sent =
            count_json_matches(heartbeat_packets, "target", &peer.endpoint);
        let heartbeat_ack_packets_received =
            count_json_matches(heartbeat_ack_packets, "peer", &peer.endpoint);
        let heartbeat_ack_packets_sent =
            count_json_matches(heartbeat_ack_packets, "target", &peer.endpoint);
        let forwarded_packets_sent =
            count_json_matches(forwarded_packets, "target", &peer.endpoint);
        let tunnel_packets_received = count_json_matches(tunnel_packets, "peer", &peer.endpoint);
        let last_seen_at_ms = max_json_u64(tunnel_packets, "peer", &peer.endpoint, "receivedAtMs");
        let last_sent_at_ms = max_optional_u64(
            max_json_u64(heartbeat_packets, "target", &peer.endpoint, "sentAtMs"),
            max_optional_u64(
                max_json_u64(heartbeat_ack_packets, "target", &peer.endpoint, "sentAtMs"),
                max_json_u64(forwarded_packets, "target", &peer.endpoint, "sentAtMs"),
            ),
        );
        let heartbeat_loss_percent = percent_unacked(
            heartbeat_packets_sent as u64,
            heartbeat_ack_packets_received as u64,
        );
        let recent_loss_window_size =
            heartbeat_loss_window_size(heartbeat_packets, "target", &peer.endpoint, 10);
        let heartbeat_loss_window_percent = heartbeat_loss_window_percent(
            heartbeat_packets,
            heartbeat_ack_packets,
            "target",
            "peer",
            &peer.endpoint,
            recent_loss_window_size,
        );
        let heartbeat_rtt_sample_count =
            count_json_u64(heartbeat_ack_packets, "peer", &peer.endpoint, "roundTripMs");
        let heartbeat_rtt_jitter_ms =
            round_trip_jitter_ms(heartbeat_ack_packets, "peer", &peer.endpoint);
        let path_kind = runtime_peer_path_kind(selected_path, connection_path_status);
        let direct_bytes_sent = if path_kind == "direct" { bytes_sent } else { 0 };
        let direct_bytes_received = if path_kind == "direct" {
            bytes_received
        } else {
            0
        };
        let relay_bytes_sent = if path_kind == "relay" { bytes_sent } else { 0 };
        let relay_bytes_received = if path_kind == "relay" {
            bytes_received
        } else {
            0
        };
        let unknown_path_bytes_sent = if path_kind == "unknown" {
            bytes_sent
        } else {
            0
        };
        let unknown_path_bytes_received = if path_kind == "unknown" {
            bytes_received
        } else {
            0
        };
        let connected = tunnel_packets_received > 0;
        let health = runtime_peer_summary_health(
            selected_path,
            connection_path_status,
            connected,
            heartbeat_ack_packets_received,
            heartbeat_loss_percent,
            heartbeat_loss_window_percent,
            latency_ms,
            heartbeat_rtt_jitter_ms,
        );
        serde_json::json!({
            "peerId": peer.peer_id,
            "virtualIp": peer.virtual_ip,
            "endpoint": peer.endpoint,
            "selectedPath": selected_path,
            "pathKind": path_kind,
            "connectionPathStatus": connection_path_status,
            "bootstrapStatus": bootstrap_status,
            "connected": connected,
            "health": health,
            "latencyMs": latency_ms,
            "lastSeenAtMs": last_seen_at_ms,
            "lastSentAtMs": last_sent_at_ms,
            "bytesSent": bytes_sent,
            "bytesReceived": bytes_received,
            "directBytesSent": direct_bytes_sent,
            "directBytesReceived": direct_bytes_received,
            "relayBytesSent": relay_bytes_sent,
            "relayBytesReceived": relay_bytes_received,
            "unknownPathBytesSent": unknown_path_bytes_sent,
            "unknownPathBytesReceived": unknown_path_bytes_received,
            "heartbeatPacketsSent": heartbeat_packets_sent,
            "heartbeatAckPacketsReceived": heartbeat_ack_packets_received,
            "heartbeatAckPacketsSent": heartbeat_ack_packets_sent,
            "heartbeatLossPercent": heartbeat_loss_percent,
            "heartbeatLossWindowSize": recent_loss_window_size,
            "heartbeatLossWindowPercent": heartbeat_loss_window_percent,
            "heartbeatRttSampleCount": heartbeat_rtt_sample_count,
            "heartbeatRttJitterMs": heartbeat_rtt_jitter_ms,
            "forwardedPacketsSent": forwarded_packets_sent,
            "tunnelPacketsReceived": tunnel_packets_received,
        })
    })
    .collect()
}

fn runtime_observed_peers(
    plan: &RoomRuntimePlan,
    tunnel_packets: &[serde_json::Value],
    heartbeat_packets: &[serde_json::Value],
    heartbeat_ack_packets: &[serde_json::Value],
    local_endpoint: Option<&str>,
) -> Vec<RoomRuntimePeer> {
    if !plan.peers.is_empty() {
        return plan.peers.clone();
    }
    let mut endpoints = tunnel_packets
        .iter()
        .filter_map(|packet| packet.get("peer").and_then(serde_json::Value::as_str))
        .chain(
            heartbeat_packets
                .iter()
                .filter_map(|packet| packet.get("target").and_then(serde_json::Value::as_str)),
        )
        .chain(
            heartbeat_ack_packets
                .iter()
                .filter_map(|packet| packet.get("peer").and_then(serde_json::Value::as_str)),
        )
        .chain(
            heartbeat_ack_packets
                .iter()
                .filter_map(|packet| packet.get("target").and_then(serde_json::Value::as_str)),
        )
        .map(str::to_owned)
        .filter(|endpoint| !endpoint.trim().is_empty())
        .collect::<Vec<_>>();
    endpoints.sort();
    endpoints.dedup();
    endpoints
        .into_iter()
        .enumerate()
        .map(|(index, endpoint)| RoomRuntimePeer {
            peer_id: if local_endpoint == Some(endpoint.as_str()) || endpoint == "self" {
                format!("{}-self-probe", plan.local_peer_id)
            } else {
                format!("observed-peer-{}", index + 1)
            },
            virtual_ip: plan.local_virtual_ip,
            endpoint,
        })
        .collect()
}

fn runtime_peer_observations_from_summaries(
    summaries: &[serde_json::Value],
) -> Vec<lai_core::RuntimePeerObservation> {
    summaries
        .iter()
        .filter_map(|summary| {
            let peer_id = summary.get("peerId")?.as_str()?.to_owned();
            Some(lai_core::RuntimePeerObservation {
                peer_id,
                virtual_ip: summary
                    .get("virtualIp")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                selected_path: summary
                    .get("selectedPath")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                connection_path_status: summary
                    .get("connectionPathStatus")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                bootstrap_status: summary
                    .get("bootstrapStatus")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                connected: summary
                    .get("connected")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                path_kind: summary
                    .get("pathKind")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                latency_ms: summary.get("latencyMs").and_then(serde_json::Value::as_u64),
                last_seen_at_ms: summary
                    .get("lastSeenAtMs")
                    .and_then(serde_json::Value::as_u64),
                last_sent_at_ms: summary
                    .get("lastSentAtMs")
                    .and_then(serde_json::Value::as_u64),
                bytes_sent: summary
                    .get("bytesSent")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                bytes_received: summary
                    .get("bytesReceived")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                direct_bytes_sent: summary
                    .get("directBytesSent")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                direct_bytes_received: summary
                    .get("directBytesReceived")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                relay_bytes_sent: summary
                    .get("relayBytesSent")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                relay_bytes_received: summary
                    .get("relayBytesReceived")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                unknown_path_bytes_sent: summary
                    .get("unknownPathBytesSent")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                unknown_path_bytes_received: summary
                    .get("unknownPathBytesReceived")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                heartbeat_packets_sent: summary
                    .get("heartbeatPacketsSent")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                heartbeat_ack_packets_received: summary
                    .get("heartbeatAckPacketsReceived")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                heartbeat_loss_percent: summary
                    .get("heartbeatLossPercent")
                    .and_then(serde_json::Value::as_f64),
                heartbeat_loss_window_size: summary
                    .get("heartbeatLossWindowSize")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default() as usize,
                heartbeat_loss_window_percent: summary
                    .get("heartbeatLossWindowPercent")
                    .and_then(serde_json::Value::as_f64),
                heartbeat_rtt_sample_count: summary
                    .get("heartbeatRttSampleCount")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default() as usize,
                heartbeat_rtt_jitter_ms: summary
                    .get("heartbeatRttJitterMs")
                    .and_then(serde_json::Value::as_f64),
                forwarded_packets_sent: summary
                    .get("forwardedPacketsSent")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                tunnel_packets_received: summary
                    .get("tunnelPacketsReceived")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
            })
        })
        .collect()
}

fn runtime_peer_summary_health(
    selected_path: &str,
    connection_path_status: &str,
    connected: bool,
    heartbeat_ack_packets_received: usize,
    heartbeat_loss_percent: Option<f64>,
    heartbeat_loss_window_percent: Option<f64>,
    latency_ms: Option<u64>,
    heartbeat_rtt_jitter_ms: Option<f64>,
) -> serde_json::Value {
    let (status, reason, next_action) = if matches!(
        connection_path_status,
        "no-path" | "needs-relay" | "config-error"
    ) || matches!(selected_path, "none" | "failed")
    {
        (
            "needs-attention",
            "no usable path",
            "Refresh NAT candidates or configure relay before starting the game.",
        )
    } else if !connected && heartbeat_ack_packets_received == 0 {
        (
            "needs-attention",
            "missing runtime packets",
            "Check that the peer runtime is still running and reachable on its tunnel endpoint.",
        )
    } else if heartbeat_loss_window_percent.is_some_and(|loss| loss >= 50.0) {
        (
            "needs-attention",
            "high recent heartbeat loss",
            "Check firewall, NAT mapping, or relay fallback; recent heartbeat acknowledgements are missing.",
        )
    } else if heartbeat_loss_percent.is_some_and(|loss| loss >= 50.0) {
        (
            "needs-attention",
            "high heartbeat loss",
            "Check firewall, NAT mapping, or relay fallback; heartbeat acknowledgements are missing.",
        )
    } else if latency_ms.is_some_and(|latency| latency >= 150) {
        (
            "degraded",
            "high latency",
            "Direct IP may work, but expect delay; consider relay region or network changes.",
        )
    } else if heartbeat_rtt_jitter_ms.is_some_and(|jitter| jitter >= 50.0) {
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
    serde_json::json!({
        "status": status,
        "reason": reason,
        "nextAction": next_action,
    })
}

fn runtime_heartbeat_ack_payload(
    plan: &RoomRuntimePlan,
    plaintext: &[u8],
    received_at_ms: u128,
) -> Result<(serde_json::Value, u64, u64), Box<dyn std::error::Error>> {
    let heartbeat: serde_json::Value = serde_json::from_slice(plaintext)?;
    let sequence = heartbeat
        .get("sequence")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| invalid_input("runtime heartbeat is missing sequence".to_owned()))?;
    let heartbeat_sent_at_ms = heartbeat
        .get("sent_at_ms")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default();
    let source_peer_id = heartbeat
        .get("peer_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    Ok((
        serde_json::json!({
            "room_id": plan.room_id,
            "peer_id": plan.local_peer_id,
            "virtual_ip": plan.local_virtual_ip,
            "kind": "runtime-heartbeat-ack",
            "ack_sequence": sequence,
            "ack_peer_id": source_peer_id,
            "heartbeat_sent_at_ms": heartbeat_sent_at_ms,
            "received_at_ms": received_at_ms,
            "sent_at_ms": current_epoch_ms(),
        }),
        sequence,
        heartbeat_sent_at_ms,
    ))
}

fn runtime_heartbeat_ack_observation(
    peer: SocketAddr,
    bytes_received: usize,
    plaintext: &[u8],
    received_at_ms: u128,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let ack: serde_json::Value = serde_json::from_slice(plaintext)?;
    let heartbeat_sent_at_ms = ack
        .get("heartbeat_sent_at_ms")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default();
    let round_trip_ms = if heartbeat_sent_at_ms == 0 {
        None
    } else {
        Some(received_at_ms.saturating_sub(heartbeat_sent_at_ms as u128))
    };
    Ok(serde_json::json!({
        "direction": "received",
        "peer": peer.to_string(),
        "bytesReceived": bytes_received,
        "ackedSequence": ack.get("ack_sequence").and_then(serde_json::Value::as_u64),
        "heartbeatSentAtMs": heartbeat_sent_at_ms,
        "ackSentAtMs": ack.get("sent_at_ms").and_then(serde_json::Value::as_u64),
        "receivedAtMs": received_at_ms,
        "roundTripMs": round_trip_ms,
    }))
}

fn percent_unacked(sent: u64, acked: u64) -> Option<f64> {
    if sent == 0 {
        None
    } else {
        let lost = sent.saturating_sub(acked);
        Some((lost as f64 / sent as f64) * 100.0)
    }
}

fn runtime_peer_path_kind(selected_path: &str, connection_path_status: &str) -> &'static str {
    if selected_path.eq_ignore_ascii_case("relay")
        || selected_path.eq_ignore_ascii_case("relayed")
        || connection_path_status.eq_ignore_ascii_case("relay-ready")
    {
        "relay"
    } else if selected_path.eq_ignore_ascii_case("p2p")
        || selected_path.eq_ignore_ascii_case("direct")
        || connection_path_status.eq_ignore_ascii_case("p2p-candidate-ready")
        || connection_path_status.eq_ignore_ascii_case("observed")
    {
        "direct"
    } else {
        "unknown"
    }
}

fn heartbeat_loss_window_size(
    sent_packets: &[serde_json::Value],
    match_key: &str,
    endpoint: &str,
    max_window_size: usize,
) -> usize {
    sent_packets
        .iter()
        .filter(|packet| {
            packet.get(match_key).and_then(serde_json::Value::as_str) == Some(endpoint)
        })
        .count()
        .min(max_window_size)
}

fn heartbeat_loss_window_percent(
    sent_packets: &[serde_json::Value],
    ack_packets: &[serde_json::Value],
    sent_match_key: &str,
    ack_match_key: &str,
    endpoint: &str,
    window_size: usize,
) -> Option<f64> {
    if window_size == 0 {
        return None;
    }
    let mut sent = sent_packets
        .iter()
        .filter(|packet| {
            packet
                .get(sent_match_key)
                .and_then(serde_json::Value::as_str)
                == Some(endpoint)
        })
        .filter_map(|packet| {
            Some((
                packet
                    .get("sentAtMs")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                packet.get("sequence").and_then(serde_json::Value::as_u64)?,
            ))
        })
        .collect::<Vec<_>>();
    sent.sort_by_key(|(sent_at_ms, sequence)| (*sent_at_ms, *sequence));
    let window = sent
        .iter()
        .rev()
        .take(window_size)
        .map(|(_, sequence)| *sequence)
        .collect::<HashSet<_>>();
    let acked = ack_packets
        .iter()
        .filter(|packet| {
            packet
                .get(ack_match_key)
                .and_then(serde_json::Value::as_str)
                == Some(endpoint)
        })
        .filter_map(|packet| {
            packet
                .get("ackedSequence")
                .and_then(serde_json::Value::as_u64)
        })
        .filter(|sequence| window.contains(sequence))
        .collect::<HashSet<_>>()
        .len() as u64;
    percent_unacked(window.len() as u64, acked)
}

fn round_trip_jitter_ms(
    values: &[serde_json::Value],
    match_key: &str,
    endpoint: &str,
) -> Option<f64> {
    let mut samples = values
        .iter()
        .filter(|value| value.get(match_key).and_then(serde_json::Value::as_str) == Some(endpoint))
        .filter_map(|value| {
            Some((
                value
                    .get("receivedAtMs")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
                value
                    .get("roundTripMs")
                    .and_then(serde_json::Value::as_u64)?,
            ))
        })
        .collect::<Vec<_>>();
    if samples.len() < 2 {
        return None;
    }
    samples.sort_by_key(|(received_at_ms, _)| *received_at_ms);
    let total_delta = samples
        .windows(2)
        .map(|pair| pair[1].1.abs_diff(pair[0].1))
        .sum::<u64>();
    Some(total_delta as f64 / (samples.len() - 1) as f64)
}

fn peer_has_tunnel_packets(packets: &[serde_json::Value], endpoint: &str) -> bool {
    packets
        .iter()
        .any(|packet| packet.get("peer").and_then(serde_json::Value::as_str) == Some(endpoint))
}

fn sum_json_bytes(
    values: &[serde_json::Value],
    match_key: &str,
    match_value: &str,
    bytes_key: &str,
) -> u64 {
    values
        .iter()
        .filter(|value| {
            value.get(match_key).and_then(serde_json::Value::as_str) == Some(match_value)
        })
        .filter_map(|value| value.get(bytes_key).and_then(serde_json::Value::as_u64))
        .sum()
}

fn count_json_matches(values: &[serde_json::Value], key: &str, expected: &str) -> usize {
    values
        .iter()
        .filter(|value| value.get(key).and_then(serde_json::Value::as_str) == Some(expected))
        .count()
}

fn count_json_u64(
    values: &[serde_json::Value],
    match_key: &str,
    match_value: &str,
    value_key: &str,
) -> usize {
    values
        .iter()
        .filter(|value| {
            value.get(match_key).and_then(serde_json::Value::as_str) == Some(match_value)
        })
        .filter(|value| {
            value
                .get(value_key)
                .and_then(serde_json::Value::as_u64)
                .is_some()
        })
        .count()
}

fn max_json_u64(
    values: &[serde_json::Value],
    match_key: &str,
    match_value: &str,
    value_key: &str,
) -> Option<u64> {
    values
        .iter()
        .filter(|value| {
            value.get(match_key).and_then(serde_json::Value::as_str) == Some(match_value)
        })
        .filter_map(|value| value.get(value_key).and_then(serde_json::Value::as_u64))
        .max()
}

fn max_optional_u64(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn json_array_values(value: &serde_json::Value, key: &str) -> Vec<serde_json::Value> {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
}

struct RuntimeForwardPayloadData {
    udp_payload: Vec<u8>,
    raw_ipv4_summary: Option<lai_core::VirtualIpv4PacketSummary>,
    raw_ipv4_packet: Option<VirtualUdpPacket>,
    raw_tcp_packet: Option<lai_core::VirtualTcpPacket>,
    raw_ipv4_packet_bytes: Option<Vec<u8>>,
}

fn runtime_forward_payload_data(
    plaintext: &[u8],
) -> Result<RuntimeForwardPayloadData, Box<dyn std::error::Error>> {
    let value: serde_json::Value = serde_json::from_slice(plaintext)?;
    let raw_ipv4_packet_bytes = value
        .get("raw_ipv4_packet")
        .and_then(serde_json::Value::as_str)
        .map(|encoded| {
            STANDARD_NO_PAD.decode(encoded).map_err(|err| {
                invalid_input(format!("invalid runtime raw IPv4 packet bytes: {err}"))
            })
        })
        .transpose()?;
    let raw_ipv4_summary = raw_ipv4_packet_bytes
        .as_ref()
        .map(|bytes| lai_core::parse_ipv4_packet_summary(bytes).map_err(invalid_input))
        .transpose()?;
    let raw_ipv4_packet = raw_ipv4_packet_bytes
        .as_ref()
        .map(|bytes| lai_core::parse_ipv4_udp_packet(bytes).map_err(invalid_input))
        .transpose()
        .ok()
        .flatten();
    let raw_tcp_packet = raw_ipv4_packet_bytes
        .as_ref()
        .map(|bytes| lai_core::parse_ipv4_tcp_packet(bytes).map_err(invalid_input))
        .transpose()
        .ok()
        .flatten();
    let udp_payload = if let Some(encoded) = value.get("bytes").and_then(serde_json::Value::as_str)
    {
        STANDARD_NO_PAD.decode(encoded).map_err(|err| {
            invalid_input(format!("invalid runtime UDP forward payload bytes: {err}"))
        })?
    } else if let Some(packet) = &raw_ipv4_packet {
        packet.payload.clone()
    } else if let Some(packet) = &raw_tcp_packet {
        packet.payload.clone()
    } else if raw_ipv4_summary.is_some() {
        Vec::new()
    } else {
        return Err(invalid_input(
            "runtime UDP forward payload is missing bytes and raw_ipv4_packet".to_owned(),
        ));
    };

    Ok(RuntimeForwardPayloadData {
        udp_payload,
        raw_ipv4_summary,
        raw_ipv4_packet,
        raw_tcp_packet,
        raw_ipv4_packet_bytes,
    })
}

fn runtime_virtual_udp_packet(
    plan: &RoomRuntimePlan,
    source: SocketAddr,
    destination: SocketAddr,
    payload: &[u8],
    broadcast: bool,
) -> VirtualUdpPacket {
    VirtualUdpPacket {
        source_ip: plan.local_virtual_ip,
        destination_ip: if broadcast {
            runtime_broadcast_ip(plan.local_virtual_ip)
        } else {
            plan.peers
                .first()
                .map(|peer| peer.virtual_ip)
                .unwrap_or_else(|| socket_addr_ipv4(destination))
        },
        source_port: source.port(),
        destination_port: destination.port(),
        payload: payload.to_vec(),
        broadcast,
    }
}

fn runtime_broadcast_ip(local_virtual_ip: Ipv4Addr) -> Ipv4Addr {
    let octets = local_virtual_ip.octets();
    Ipv4Addr::new(octets[0], octets[1], octets[2], 255)
}

fn loopback_target(address: SocketAddr) -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, address.port()))
}

fn socket_addr_ipv4(address: SocketAddr) -> Ipv4Addr {
    match address {
        SocketAddr::V4(address) => *address.ip(),
        SocketAddr::V6(_) => Ipv4Addr::UNSPECIFIED,
    }
}

fn is_broadcast_destination(value: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let address = value.parse::<Ipv4Addr>()?;
    Ok(address == Ipv4Addr::BROADCAST || address.octets()[3] == 255)
}

fn invalid_input(message: String) -> Box<dyn std::error::Error> {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message).into()
}
