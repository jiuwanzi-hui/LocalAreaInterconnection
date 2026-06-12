use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use clap::{CommandFactory, Parser, Subcommand};
use lai_core::{
    add_room_member, close_room, create_command_execution_preview, create_diagnostic_export_bundle,
    create_game_network_plan, create_invite, create_join_plan, create_p2p_handshake_ack,
    create_p2p_handshake_hello, create_room, create_room_runtime_plan, create_room_session,
    create_windows_firewall_plan, create_windows_virtual_adapter_ensure_report,
    create_windows_virtual_adapter_plan, decode_invite, evaluate_firewall_diagnostics,
    evaluate_network_observations, network_snapshot_from_runtime, observation_from_expected_rule,
    open_tunnel_payload, parse_netsh_adapter_observation, parse_netsh_firewall_rules,
    parse_windows_ping_observation, seal_tunnel_payload, udp_forward_summary, AdapterObservation,
    CommandExecutionRecord, CommandExecutionStatus, CompatibilityLevel,
    DiagnosticExportEnvironment, DiagnosticExportInputs, DiagnosticExportSources,
    DiagnosticSnapshot, DiagnosticTextSource, DiscoveryMode, FirewallRule, FirewallRuleObservation,
    GameProfile, Ipv4Subnet, NetworkCommand, NetworkObservationSnapshot, P2pHandshakeAck,
    P2pHandshakeHello, PacketCaptureSummary, PacketObservation, RoomRuntimePeer, RoomRuntimePlan,
    TunnelEnvelope, TunnelObservation, TunnelServiceSnapshot, UdpForwardObservation,
    VirtualUdpPacket,
};
use rand::RngCore;
use std::fs;
use std::io::{ErrorKind, Write};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(name = "lai-cli")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

struct RuntimePacketIoProbeOptions {
    wintun_adapter_name: String,
    wintun_ring_capacity: u32,
    wintun_probe_receive: bool,
    wintun_receive_attempts: u32,
    wintun_receive_poll_interval_ms: u64,
    wintun_probe_send: bool,
}

#[derive(Subcommand)]
enum Command {
    Init {
        #[arg(long, default_value = "LAN Room")]
        room_name: String,
        #[arg(long, default_value = "Host")]
        host: String,
    },
    Decode {
        #[arg(long)]
        invite: String,
    },
    Join {
        #[arg(long)]
        invite: String,
    },
    RoomSummary {
        #[arg(long, default_value = "LAN Room")]
        room_name: String,
        #[arg(long, default_value = "Host")]
        host: String,
        #[arg(long = "peer")]
        peers: Vec<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        close: bool,
    },
    RoomRuntimePlan {
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        peer_id: String,
        #[arg(long)]
        virtual_ip: String,
        #[arg(long, default_value = "0.0.0.0:39090")]
        bind: String,
        #[arg(long = "peer")]
        peers: Vec<String>,
        #[arg(long = "nat-bootstrap-peer")]
        nat_bootstrap_peers: Vec<String>,
        #[arg(long, default_value = "")]
        game_ports: String,
        #[arg(long, default_value = "")]
        broadcast_ports: String,
    },
    RoomRuntimeRun {
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        peer_id: String,
        #[arg(long)]
        virtual_ip: String,
        #[arg(long, default_value = "127.0.0.1:0")]
        bind: String,
        #[arg(long = "peer")]
        peers: Vec<String>,
        #[arg(long = "nat-bootstrap-peer")]
        nat_bootstrap_peers: Vec<String>,
        #[arg(long = "nat-bootstrap-remote-peer")]
        nat_bootstrap_remote_peers: Vec<String>,
        #[arg(long)]
        coordination_store: Option<String>,
        #[arg(long = "coordination-peer")]
        coordination_peers: Vec<String>,
        #[arg(long, default_value = "")]
        game_ports: String,
        #[arg(long, default_value = "")]
        broadcast_ports: String,
        #[arg(long)]
        key: String,
        #[arg(long, default_value_t = 1000)]
        duration_ms: u64,
        #[arg(long)]
        observe_file: Option<String>,
        #[arg(long)]
        snapshot_out: Option<String>,
        #[arg(long, default_value = "userspace-udp")]
        packet_io_backend: String,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        forward_raw_ipv4: bool,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        self_probe: bool,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        capture_self_probe: bool,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        forward_self_probe: bool,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        inject_self_probe: bool,
        #[arg(long)]
        inject_target: Option<String>,
        #[arg(long, default_value_t = 500)]
        heartbeat_interval_ms: u64,
        #[arg(long, default_value_t = 3000)]
        peer_timeout_ms: u64,
        #[arg(long, default_value_t = 4)]
        nat_bootstrap_attempts: u16,
        #[arg(long, default_value_t = 25)]
        nat_bootstrap_interval_ms: u64,
        #[arg(long, default_value_t = 2000)]
        nat_bootstrap_timeout_ms: u64,
        #[arg(long)]
        stop_file: Option<String>,
        #[arg(long)]
        snapshot_interval_ms: Option<u64>,
        #[arg(long, default_value = "LocalAreaInterconnection")]
        wintun_adapter_name: String,
        #[arg(long, default_value_t = 128 * 1024)]
        wintun_ring_capacity: u32,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        wintun_probe_receive: bool,
        #[arg(long, default_value_t = 8)]
        wintun_receive_attempts: u32,
        #[arg(long, default_value_t = 25)]
        wintun_receive_poll_interval_ms: u64,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        wintun_probe_send: bool,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        wintun_runtime: bool,
    },
    Diagnose {
        #[arg(long)]
        p2p: Option<String>,
        #[arg(long)]
        firewall: Option<String>,
    },
    GamePlan {
        #[arg(long, default_value = "Generic LAN Game")]
        game_name: String,
        #[arg(long)]
        subnet: String,
        #[arg(long, default_value = "udp_broadcast")]
        discovery: String,
        #[arg(long, default_value = "")]
        ports: String,
        #[arg(long, default_value = "unknown")]
        compatibility: String,
        #[arg(long)]
        host_ip: Option<String>,
        #[arg(long)]
        local_ip: Option<String>,
    },
    FirewallPlan {
        #[arg(long, default_value = "Generic LAN Game")]
        game_name: String,
        #[arg(long)]
        subnet: String,
        #[arg(long, default_value = "manual_ports")]
        discovery: String,
        #[arg(long, default_value = "")]
        ports: String,
        #[arg(long, default_value = "unknown")]
        compatibility: String,
        #[arg(long)]
        program: Option<String>,
    },
    FirewallDiagnose {
        #[arg(long, default_value = "Generic LAN Game")]
        game_name: String,
        #[arg(long)]
        subnet: String,
        #[arg(long, default_value = "manual_ports")]
        discovery: String,
        #[arg(long, default_value = "")]
        ports: String,
        #[arg(long, default_value = "unknown")]
        compatibility: String,
        #[arg(long, default_value = "")]
        observed: String,
        #[arg(long)]
        netsh_output: Option<String>,
        #[arg(long)]
        program: Option<String>,
    },
    AdapterPlan {
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long)]
        subnet: String,
        #[arg(long)]
        ip: String,
        #[arg(long, default_value_t = 1420)]
        mtu: u16,
        #[arg(long, default_value_t = 5)]
        metric: u16,
    },
    AdapterApply {
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long)]
        subnet: String,
        #[arg(long)]
        ip: String,
        #[arg(long, default_value_t = 1420)]
        mtu: u16,
        #[arg(long, default_value_t = 5)]
        metric: u16,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        yes: bool,
    },
    AdapterEnsure {
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long)]
        subnet: String,
        #[arg(long)]
        ip: String,
        #[arg(long, default_value_t = 1420)]
        mtu: u16,
        #[arg(long, default_value_t = 5)]
        metric: u16,
        #[arg(long)]
        adapter_netsh_output: Option<String>,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        adapter_scan: bool,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        yes: bool,
    },
    VirtualPacketPlan {
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long, default_value = "userspace-udp")]
        backend: String,
        #[arg(long, default_value_t = 1420)]
        mtu: u16,
    },
    VirtualPacketBuildUdp {
        #[arg(long)]
        source_ip: String,
        #[arg(long)]
        destination_ip: String,
        #[arg(long)]
        source_port: u16,
        #[arg(long)]
        destination_port: u16,
        #[arg(long, default_value = "hello")]
        message: String,
        #[arg(long, default_value_t = 64)]
        ttl: u8,
    },
    VirtualPacketBuildTcp {
        #[arg(long)]
        source_ip: String,
        #[arg(long)]
        destination_ip: String,
        #[arg(long)]
        source_port: u16,
        #[arg(long)]
        destination_port: u16,
        #[arg(long, default_value = "hello")]
        message: String,
        #[arg(long, default_value_t = 0x18)]
        flags: u16,
        #[arg(long, default_value_t = 64)]
        ttl: u8,
    },
    VirtualPacketParse {
        #[arg(long)]
        packet_base64: String,
    },
    VirtualPacketParseSummary {
        #[arg(long)]
        packet_base64: String,
    },
    VirtualPacketLoopbackTest {
        #[arg(long, default_value = "10.77.12.2")]
        source_ip: String,
        #[arg(long, default_value = "10.77.12.255")]
        destination_ip: String,
        #[arg(long, default_value_t = 39077)]
        source_port: u16,
        #[arg(long, default_value_t = 27015)]
        destination_port: u16,
        #[arg(long, default_value = "discover")]
        message: String,
    },
    TunnelSeal {
        #[arg(long)]
        key: String,
        #[arg(long, default_value = "game-udp")]
        packet_kind: String,
        #[arg(long, default_value_t = 1)]
        sequence: u64,
        #[arg(long)]
        message: String,
    },
    TunnelOpen {
        #[arg(long)]
        key: String,
        #[arg(long)]
        envelope: String,
    },
    TunnelLoopbackTest {
        #[arg(long, default_value = "127.0.0.1:0")]
        bind: String,
        #[arg(long)]
        key: String,
        #[arg(long, default_value = "ping")]
        message: String,
        #[arg(long, default_value_t = 2000)]
        timeout_ms: u64,
    },
    TunnelListen {
        #[arg(long, default_value = "0.0.0.0:39090")]
        bind: String,
        #[arg(long)]
        key: String,
        #[arg(long, default_value_t = 1)]
        max_packets: u16,
        #[arg(long, default_value_t = 30000)]
        timeout_ms: u64,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        echo: bool,
    },
    TunnelSend {
        #[arg(long, default_value = "0.0.0.0:0")]
        bind: String,
        #[arg(long)]
        peer: String,
        #[arg(long)]
        key: String,
        #[arg(long, default_value = "ping")]
        message: String,
        #[arg(long, default_value_t = 2000)]
        timeout_ms: u64,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        wait_reply: bool,
    },
    P2pHandshakeLoopbackTest {
        #[arg(long, default_value = "127.0.0.1:0")]
        bind: String,
        #[arg(long, default_value = "room_test")]
        room_id: String,
        #[arg(long, default_value = "peer_local")]
        peer_id: String,
        #[arg(long, default_value = "peer_echo")]
        responder_peer_id: String,
        #[arg(long)]
        virtual_ip: String,
        #[arg(long)]
        key: String,
        #[arg(long, default_value_t = 2000)]
        timeout_ms: u64,
    },
    P2pHandshakeListen {
        #[arg(long, default_value = "0.0.0.0:39090")]
        bind: String,
        #[arg(long)]
        key: String,
        #[arg(long, default_value = "peer_responder")]
        responder_peer_id: String,
        #[arg(long, default_value_t = 1)]
        max_packets: u16,
        #[arg(long, default_value_t = 30000)]
        timeout_ms: u64,
    },
    P2pHandshakeSend {
        #[arg(long, default_value = "0.0.0.0:0")]
        bind: String,
        #[arg(long)]
        peer: String,
        #[arg(long, default_value = "room_test")]
        room_id: String,
        #[arg(long, default_value = "peer_local")]
        peer_id: String,
        #[arg(long)]
        virtual_ip: String,
        #[arg(long)]
        key: String,
        #[arg(long, default_value_t = 2000)]
        timeout_ms: u64,
    },
    NatCandidates {
        #[arg(long, default_value = "room_test")]
        room_id: String,
        #[arg(long, default_value = "peer_local")]
        peer_id: String,
        #[arg(long, default_value = "0.0.0.0:0")]
        bind: String,
        #[arg(long)]
        observed_endpoint: Option<String>,
        #[arg(long = "relay")]
        relay_endpoints: Vec<String>,
        #[arg(long)]
        nonce: Option<String>,
    },
    NatPlan {
        #[arg(long)]
        local_offer: String,
        #[arg(long)]
        remote_offer: String,
        #[arg(long, default_value_t = 8)]
        attempts: u16,
        #[arg(long, default_value_t = 50)]
        interval_ms: u64,
    },
    NatHolePunch {
        #[arg(long, default_value = "room_test")]
        room_id: String,
        #[arg(long, default_value = "peer_local")]
        peer_id: String,
        #[arg(long, default_value = "0.0.0.0:0")]
        bind: String,
        #[arg(long)]
        remote_offer: String,
        #[arg(long)]
        observed_endpoint: Option<String>,
        #[arg(long = "relay")]
        relay_endpoints: Vec<String>,
        #[arg(long, default_value_t = 8)]
        attempts: u16,
        #[arg(long, default_value_t = 50)]
        interval_ms: u64,
        #[arg(long, default_value_t = 500)]
        receive_timeout_ms: u64,
        #[arg(long, default_value = "nat-punch")]
        message: String,
    },
    NatP2pBootstrap {
        #[arg(long, default_value = "room_test")]
        room_id: String,
        #[arg(long, default_value = "peer_local")]
        peer_id: String,
        #[arg(long)]
        virtual_ip: String,
        #[arg(long)]
        key: String,
        #[arg(long, default_value = "0.0.0.0:0")]
        bind: String,
        #[arg(long)]
        remote_offer: String,
        #[arg(long)]
        observed_endpoint: Option<String>,
        #[arg(long = "relay")]
        relay_endpoints: Vec<String>,
        #[arg(long, default_value_t = 4)]
        punch_attempts: u16,
        #[arg(long, default_value_t = 25)]
        punch_interval_ms: u64,
        #[arg(long, default_value_t = 2000)]
        handshake_timeout_ms: u64,
    },
    NatHolePunchLoopbackTest {
        #[arg(long, default_value = "room_test")]
        room_id: String,
        #[arg(long, default_value = "peer_a")]
        peer_a: String,
        #[arg(long, default_value = "peer_b")]
        peer_b: String,
        #[arg(long, default_value_t = 4)]
        attempts: u16,
        #[arg(long, default_value_t = 25)]
        interval_ms: u64,
        #[arg(long, default_value = "nat-punch")]
        message: String,
    },
    CoordinationStoreInit {
        #[arg(long)]
        out: String,
    },
    CoordinationOfferPublish {
        #[arg(long)]
        store: String,
        #[arg(long)]
        offer: String,
        #[arg(long, default_value_t = 30000)]
        ttl_ms: u128,
    },
    CoordinationOfferFetch {
        #[arg(long)]
        store: String,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        peer_id: String,
    },
    CoordinationHeartbeat {
        #[arg(long)]
        store: String,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        peer_id: String,
        #[arg(long, default_value_t = 30000)]
        ttl_ms: u128,
    },
    CoordinationPrune {
        #[arg(long)]
        store: String,
    },
    UdpForward {
        #[arg(long, default_value = "0.0.0.0:39078")]
        listen: String,
        #[arg(long)]
        forward: String,
        #[arg(long, default_value_t = 64)]
        max_packets: u16,
        #[arg(long, default_value_t = 30000)]
        timeout_ms: u64,
        #[arg(long)]
        observe_file: Option<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        broadcast: bool,
    },
    UdpForwardLoopbackTest {
        #[arg(long, default_value = "hello")]
        message: String,
        #[arg(long)]
        observe_file: Option<String>,
    },
    UdpCapture {
        #[arg(long, default_value = "0.0.0.0:39077")]
        listen: String,
        #[arg(long, default_value_t = 64)]
        max_packets: u16,
        #[arg(long, default_value_t = 30000)]
        timeout_ms: u64,
        #[arg(long)]
        observe_file: Option<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        broadcast: bool,
    },
    UdpCaptureLoopbackTest {
        #[arg(long, default_value = "hello")]
        message: String,
        #[arg(long)]
        observe_file: Option<String>,
    },
    NetworkObserve {
        #[arg(long)]
        adapter_name: Option<String>,
        #[arg(long, default_value_t = true)]
        adapter_enabled: bool,
        #[arg(long)]
        expected_ip: Option<String>,
        #[arg(long)]
        assigned_ip: Option<String>,
        #[arg(long)]
        subnet: Option<String>,
        #[arg(long)]
        adapter_netsh_output: Option<String>,
        #[arg(long, default_value = "connected")]
        tunnel_state: String,
        #[arg(long, default_value_t = 0)]
        connected_peers: u16,
        #[arg(long, default_value_t = 0)]
        expected_peers: u16,
        #[arg(long)]
        latency_ms: Option<u32>,
        #[arg(long)]
        packet_loss_percent: Option<f32>,
        #[arg(long)]
        ping_output: Option<String>,
        #[arg(long, default_value = "")]
        broadcast_ports: String,
        #[arg(long, default_value = "")]
        game_ports: String,
        #[arg(long, default_value = "")]
        packets: String,
        #[arg(long)]
        packet_observations: Option<String>,
    },
    DiagnosticExport {
        #[arg(long)]
        out: String,
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long)]
        expected_ip: Option<String>,
        #[arg(long)]
        assigned_ip: Option<String>,
        #[arg(long)]
        subnet: Option<String>,
        #[arg(long)]
        adapter_netsh_output: Option<String>,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        adapter_scan: bool,
        #[arg(long)]
        firewall_netsh_output: Option<String>,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        firewall_scan: bool,
        #[arg(long)]
        ping_test: Option<String>,
        #[arg(long)]
        ping_output: Option<String>,
        #[arg(long, default_value_t = 0)]
        expected_peers: u16,
        #[arg(long, default_value = "")]
        broadcast_ports: String,
        #[arg(long, default_value = "")]
        game_ports: String,
        #[arg(long, default_value = "")]
        packets: String,
        #[arg(long)]
        packet_observations: Option<String>,
        #[arg(long, default_value = "Generic LAN Game")]
        game_name: String,
        #[arg(long, default_value = "manual_ports")]
        discovery: String,
        #[arg(long, default_value = "")]
        ports: String,
        #[arg(long, default_value = "unknown")]
        compatibility: String,
        #[arg(long)]
        program: Option<String>,
        #[arg(long, default_value = "userspace-udp")]
        packet_io_backend: String,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        packet_io_probe: bool,
        #[arg(long, default_value_t = 128 * 1024)]
        wintun_ring_capacity: u32,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        wintun_probe_receive: bool,
        #[arg(long, default_value_t = 8)]
        wintun_receive_attempts: u32,
        #[arg(long, default_value_t = 25)]
        wintun_receive_poll_interval_ms: u64,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        wintun_probe_send: bool,
    },
    WintunDetect,
    WintunAdapterCreate {
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long, default_value = "LocalAreaInterconnection")]
        tunnel_type: String,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        yes: bool,
    },
    WintunAdapterDelete {
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long, default_value = "LocalAreaInterconnection")]
        tunnel_type: String,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        force_close_sessions: bool,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        yes: bool,
    },
    WintunAdapterOpen {
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long, default_value = "LocalAreaInterconnection")]
        tunnel_type: String,
    },
    WintunSessionProbe {
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long, default_value = "LocalAreaInterconnection")]
        tunnel_type: String,
        #[arg(long, default_value_t = 128 * 1024)]
        ring_capacity: u32,
    },
    WintunPacketSendProbe {
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long, default_value_t = 128 * 1024)]
        ring_capacity: u32,
        #[arg(long, default_value = "10.77.12.2")]
        source_ip: String,
        #[arg(long, default_value = "10.77.12.255")]
        destination_ip: String,
        #[arg(long, default_value_t = 39077)]
        source_port: u16,
        #[arg(long, default_value_t = 27015)]
        destination_port: u16,
        #[arg(long, default_value = "wintun-probe")]
        message: String,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        broadcast: bool,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        yes: bool,
    },
    WintunPacketReceiveProbe {
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long, default_value_t = 128 * 1024)]
        ring_capacity: u32,
        #[arg(long, default_value_t = 8)]
        max_attempts: u32,
        #[arg(long, default_value_t = 25)]
        poll_interval_ms: u64,
    },
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
        Command::RoomRuntimeRun {
            room_id,
            peer_id,
            virtual_ip,
            bind,
            peers,
            nat_bootstrap_peers,
            nat_bootstrap_remote_peers,
            coordination_store,
            coordination_peers,
            game_ports,
            broadcast_ports,
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
            stop_file,
            snapshot_interval_ms,
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
            )?;
            runtime_peers.append(&mut bootstrapped_peers);
            let (mut coordination_bootstrapped_peers, mut coordination_bootstrap_results) =
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
                )?;
            runtime_peers.append(&mut coordination_bootstrapped_peers);
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
                &RuntimePacketIoProbeOptions {
                    wintun_adapter_name,
                    wintun_ring_capacity,
                    wintun_probe_receive,
                    wintun_receive_attempts,
                    wintun_receive_poll_interval_ms,
                    wintun_probe_send,
                },
                wintun_runtime,
                broadcast_ports,
                game_ports,
            )?;
            result["natBootstrapResults"] = serde_json::Value::Array(nat_bootstrap_results);
            result["coordinationBootstrapResults"] =
                serde_json::Value::Array(std::mem::take(&mut coordination_bootstrap_results));
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
        Command::FirewallPlan {
            game_name,
            subnet,
            discovery,
            ports,
            compatibility,
            program,
        } => {
            let profile = profile_from_args(game_name, discovery, ports, compatibility)?;
            let subnet = subnet.parse::<Ipv4Subnet>()?;
            let network_plan = create_game_network_plan(&profile, subnet, None, None, 30);
            let firewall_plan = create_windows_firewall_plan(
                &network_plan.firewall_rules,
                "LocalAreaInterconnection",
                program,
            );
            println!("{}", serde_json::to_string_pretty(&firewall_plan)?);
        }
        Command::FirewallDiagnose {
            game_name,
            subnet,
            discovery,
            ports,
            compatibility,
            observed,
            netsh_output,
            program,
        } => {
            let profile = profile_from_args(game_name, discovery, ports, compatibility)?;
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
            bind,
            observed_endpoint,
            relay_endpoints,
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
            let offer = lai_core::create_nat_traversal_offer(
                &room_id,
                &peer_id,
                nonce.unwrap_or_else(random_nonce),
                current_epoch_ms(),
                local_endpoint,
                observed_endpoint,
                relay_endpoints,
            );
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
        Command::NatHolePunch {
            room_id,
            peer_id,
            bind,
            remote_offer,
            observed_endpoint,
            relay_endpoints,
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
            relay_endpoints,
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
        Command::CoordinationPrune { store } => {
            let mut coordination_store = load_coordination_store_or_default(&store)?;
            let report = lai_core::prune_expired_coordination_peers(
                &mut coordination_store,
                current_epoch_ms(),
            );
            write_json_file(&store, &coordination_store)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
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
        Command::NetworkObserve {
            adapter_name,
            adapter_enabled,
            expected_ip,
            assigned_ip,
            subnet,
            adapter_netsh_output,
            tunnel_state,
            connected_peers,
            expected_peers,
            latency_ms,
            packet_loss_percent,
            ping_output,
            broadcast_ports,
            game_ports,
            packets,
            packet_observations,
        } => {
            let expected_ip = parse_optional_ipv4(expected_ip.as_deref())?;
            let expected_subnet = parse_optional_subnet(subnet.as_deref())?;
            let adapter = if let Some(path) = adapter_netsh_output {
                parse_netsh_adapter_observation(
                    adapter_name.unwrap_or_else(|| "LocalAreaInterconnection".to_owned()),
                    &fs::read_to_string(path)?,
                    expected_ip,
                    expected_subnet,
                )
            } else {
                adapter_name
                    .map(|adapter_name| {
                        Ok::<_, Box<dyn std::error::Error>>(AdapterObservation {
                            adapter_name,
                            enabled: adapter_enabled,
                            expected_ip,
                            assigned_ip: parse_optional_ipv4(assigned_ip.as_deref())?,
                            virtual_subnet: expected_subnet,
                            mtu: None,
                            interface_metric: None,
                        })
                    })
                    .transpose()?
            };
            let mut packet_observations_data = if let Some(path) = packet_observations {
                lai_core::parse_packet_observation_lines(&fs::read_to_string(path)?)?
            } else {
                Vec::new()
            };
            packet_observations_data.extend(parse_packet_observations(&packets)?);
            let report = evaluate_network_observations(NetworkObservationSnapshot {
                adapter,
                tunnel: Some(if let Some(path) = ping_output {
                    parse_windows_ping_observation(&fs::read_to_string(path)?, expected_peers)
                } else {
                    TunnelObservation {
                        state: tunnel_state,
                        connected_peer_count: connected_peers,
                        latency_ms,
                        packet_loss_percent,
                        path: None,
                    }
                }),
                packets: packet_observations_data,
                expected_peer_count: expected_peers,
                expected_broadcast_ports: parse_ports(&broadcast_ports)?,
                expected_game_ports: parse_ports(&game_ports)?,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
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
            game_name,
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
        } => {
            let expected_ip = parse_optional_ipv4(expected_ip.as_deref())?;
            let assigned_ip = parse_optional_ipv4(assigned_ip.as_deref())?;
            let subnet = parse_optional_subnet(subnet.as_deref())?;
            let broadcast_ports = parse_ports(&broadcast_ports)?;
            let game_ports = parse_ports(&game_ports)?;
            let packet_observations_path = packet_observations.clone();
            let packet_data = load_packet_observations(packet_observations.as_deref(), &packets);
            let packet_io_plan =
                lai_core::create_virtual_packet_io_plan(&adapter_name, &packet_io_backend, 1420);
            let packet_io_probe_value = packet_io_probe.then(|| {
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
            });
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
                game_name,
                discovery: parse_discovery(&discovery)?,
                ports: parse_ports(&ports)?,
                compatibility: parse_compatibility(&compatibility)?,
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
                packet_io_plan: Some(serde_json::to_value(packet_io_plan)?),
                packet_io_probe: packet_io_probe_value,
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
    let selected = result
        .get("selectedPeer")
        .ok_or_else(|| invalid_input("NAT bootstrap result is missing selectedPeer".to_owned()))?;
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
    if !selected
        .get("accepted")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return Err(invalid_input(
            "NAT bootstrap selectedPeer was not accepted by the responder".to_owned(),
        ));
    }
    if !selected
        .get("nonceMatched")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return Err(invalid_input(
            "NAT bootstrap selectedPeer nonce did not match the handshake".to_owned(),
        ));
    }
    let endpoint = selected
        .get("endpoint")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_input("NAT bootstrap selectedPeer is missing endpoint".to_owned()))?
        .to_owned();

    Ok(RoomRuntimePeer {
        peer_id: peer_id.to_owned(),
        virtual_ip,
        endpoint,
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
        fs::read_to_string(value)?
    } else {
        value.to_owned()
    };
    Ok(serde_json::from_str(&text)?)
}

fn load_coordination_store_or_default(
    path: &str,
) -> Result<lai_core::CoordinationStore, Box<dyn std::error::Error>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(lai_core::create_coordination_store()),
        Err(err) => Err(err.into()),
    }
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
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
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

fn execute_network_commands(commands: &[NetworkCommand]) -> Vec<CommandExecutionRecord> {
    commands.iter().map(execute_network_command).collect()
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

fn load_nat_offer_argument(
    value: &str,
) -> Result<lai_core::NatTraversalOffer, Box<dyn std::error::Error>> {
    let text = if Path::new(value).exists() {
        fs::read_to_string(value)?
    } else {
        value.to_owned()
    };
    Ok(serde_json::from_str(&text)?)
}

fn run_nat_hole_punch(
    room_id: &str,
    peer_id: &str,
    bind: &str,
    remote_offer: &lai_core::NatTraversalOffer,
    observed_endpoint: Option<SocketAddr>,
    relay_endpoints: Vec<SocketAddr>,
    attempts: u16,
    interval_ms: u64,
    receive_timeout_ms: u64,
    message: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind(bind)?;
    if receive_timeout_ms > 0 {
        socket.set_read_timeout(Some(Duration::from_millis(receive_timeout_ms)))?;
    }
    let local_offer = lai_core::create_nat_traversal_offer(
        room_id,
        peer_id,
        random_nonce(),
        current_epoch_ms(),
        socket.local_addr()?,
        observed_endpoint,
        relay_endpoints,
    );
    let plan = lai_core::create_nat_punch_plan(&local_offer, remote_offer, attempts, interval_ms);
    let mut sent_packets = Vec::new();
    let mut received_packets = Vec::new();
    let mut buffer = [0u8; 2048];

    if plan.status == "ready" {
        for attempt in 0..plan.attempt_count {
            let payload = serde_json::json!({
                "schemaVersion": 1,
                "type": "nat-punch",
                "roomId": room_id,
                "peerId": peer_id,
                "attempt": attempt,
                "message": message,
                "sentAtMs": current_epoch_ms(),
            })
            .to_string();
            for target in &plan.target_endpoints {
                let sent = socket.send_to(payload.as_bytes(), target)?;
                sent_packets.push(serde_json::json!({
                    "target": target,
                    "attempt": attempt,
                    "bytes": sent,
                }));
            }
            if receive_timeout_ms > 0 {
                drain_udp_socket_packet_records(&socket, &mut buffer, &mut received_packets)?;
            }
            if interval_ms > 0 && attempt + 1 < plan.attempt_count {
                std::thread::sleep(Duration::from_millis(interval_ms));
            }
        }
        if receive_timeout_ms > 0 {
            drain_udp_socket_packet_records(&socket, &mut buffer, &mut received_packets)?;
        }
    }

    let status = if plan.status != "ready" {
        plan.status.clone()
    } else if received_packets.is_empty() {
        "sent-no-response".to_owned()
    } else {
        "ok".to_owned()
    };

    Ok(serde_json::json!({
        "status": status,
        "localOffer": local_offer,
        "remoteOffer": remote_offer,
        "plan": plan,
        "sentPackets": sent_packets,
        "receivedPackets": received_packets,
    }))
}

fn run_nat_p2p_bootstrap(
    room_id: &str,
    peer_id: &str,
    virtual_ip: Ipv4Addr,
    key: &str,
    bind: &str,
    remote_offer: &lai_core::NatTraversalOffer,
    observed_endpoint: Option<SocketAddr>,
    relay_endpoints: Vec<SocketAddr>,
    punch_attempts: u16,
    punch_interval_ms: u64,
    handshake_timeout_ms: u64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind(bind)?;
    socket.set_read_timeout(Some(Duration::from_millis(25)))?;
    let local_endpoint = socket.local_addr()?;
    let local_offer = lai_core::create_nat_traversal_offer(
        room_id,
        peer_id,
        random_nonce(),
        current_epoch_ms(),
        local_endpoint,
        observed_endpoint,
        relay_endpoints,
    );
    let plan = lai_core::create_nat_punch_plan(
        &local_offer,
        remote_offer,
        punch_attempts,
        punch_interval_ms,
    );
    let mut punch_packets = Vec::new();
    let mut handshake_packets = Vec::new();
    let mut ignored_packets = Vec::new();
    let mut selected_peer = None;
    let mut buffer = vec![0u8; 65_535];

    if plan.status == "ready" {
        for attempt in 0..plan.attempt_count {
            let payload = serde_json::json!({
                "schemaVersion": 1,
                "type": "nat-punch",
                "roomId": room_id,
                "peerId": peer_id,
                "attempt": attempt,
                "sentAtMs": current_epoch_ms(),
            })
            .to_string();
            for target in &plan.target_endpoints {
                let sent = socket.send_to(payload.as_bytes(), target)?;
                punch_packets.push(serde_json::json!({
                    "target": target,
                    "attempt": attempt,
                    "bytes": sent,
                }));
            }
            if punch_interval_ms > 0 && attempt + 1 < plan.attempt_count {
                std::thread::sleep(Duration::from_millis(punch_interval_ms));
            }
        }

        let started_at_ms = current_epoch_ms();
        let hello = create_p2p_handshake_hello(
            room_id,
            peer_id,
            virtual_ip,
            local_endpoint.to_string(),
            random_nonce(),
            started_at_ms,
        );
        let hello_bytes = serde_json::to_vec(&hello)?;
        let envelope =
            seal_tunnel_payload(key, "p2p-handshake-hello", 1, started_at_ms, &hello_bytes)?;
        let envelope_bytes = serde_json::to_vec(&envelope)?;
        for target in &plan.target_endpoints {
            let sent = socket.send_to(&envelope_bytes, target)?;
            handshake_packets.push(serde_json::json!({
                "target": target,
                "packetKind": "p2p-handshake-hello",
                "bytes": sent,
            }));
        }

        let deadline = Instant::now() + Duration::from_millis(handshake_timeout_ms);
        while handshake_timeout_ms > 0 && Instant::now() < deadline {
            match socket.recv_from(&mut buffer) {
                Ok((received, peer)) => {
                    let envelope: TunnelEnvelope = match serde_json::from_slice(&buffer[..received])
                    {
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
                    if payload.metadata.packet_kind != "p2p-handshake-ack" {
                        ignored_packets.push(serde_json::json!({
                            "peer": peer.to_string(),
                            "bytes": received,
                            "reason": "unexpected-packet-kind",
                            "packetKind": payload.metadata.packet_kind,
                        }));
                        continue;
                    }
                    let ack: P2pHandshakeAck = serde_json::from_slice(&payload.plaintext)?;
                    let nonce_matched = ack.nonce == hello.nonce;
                    selected_peer = Some(serde_json::json!({
                        "endpoint": peer.to_string(),
                        "responderPeerId": ack.responder_peer_id,
                        "observedEndpoint": ack.observed_endpoint,
                        "nonceMatched": nonce_matched,
                        "accepted": ack.accepted,
                        "latencyMs": current_epoch_ms().saturating_sub(started_at_ms),
                    }));
                    if ack.accepted && nonce_matched {
                        break;
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
    }

    let status = if plan.status != "ready" {
        plan.status.clone()
    } else if selected_peer
        .as_ref()
        .and_then(|peer| peer["accepted"].as_bool())
        .unwrap_or(false)
        && selected_peer
            .as_ref()
            .and_then(|peer| peer["nonceMatched"].as_bool())
            .unwrap_or(false)
    {
        "ok".to_owned()
    } else {
        "handshake-timeout".to_owned()
    };

    Ok(serde_json::json!({
        "status": status,
        "localOffer": local_offer,
        "remoteOffer": remote_offer,
        "plan": plan,
        "punchPackets": punch_packets,
        "handshakePackets": handshake_packets,
        "ignoredPackets": ignored_packets,
        "selectedPeer": selected_peer,
    }))
}

fn run_nat_hole_punch_loopback_test(
    room_id: &str,
    peer_a: &str,
    peer_b: &str,
    attempts: u16,
    interval_ms: u64,
    message: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let socket_a = UdpSocket::bind("127.0.0.1:0")?;
    let socket_b = UdpSocket::bind("127.0.0.1:0")?;
    socket_a.set_read_timeout(Some(Duration::from_millis(50)))?;
    socket_b.set_read_timeout(Some(Duration::from_millis(50)))?;
    let offer_a = lai_core::create_nat_traversal_offer(
        room_id,
        peer_a,
        random_nonce(),
        current_epoch_ms(),
        socket_a.local_addr()?,
        Some(socket_a.local_addr()?),
        Vec::new(),
    );
    let offer_b = lai_core::create_nat_traversal_offer(
        room_id,
        peer_b,
        random_nonce(),
        current_epoch_ms(),
        socket_b.local_addr()?,
        Some(socket_b.local_addr()?),
        Vec::new(),
    );
    let plan_a = lai_core::create_nat_punch_plan(&offer_a, &offer_b, attempts, interval_ms);
    let plan_b = lai_core::create_nat_punch_plan(&offer_b, &offer_a, attempts, interval_ms);
    let mut sent_a = 0u16;
    let mut sent_b = 0u16;
    let mut received_by_a = 0u16;
    let mut received_by_b = 0u16;
    let mut buffer = [0u8; 2048];

    for attempt in 0..attempts.max(1) {
        let payload_a = format!("{}:{}:{attempt}:{message}", room_id, peer_a);
        for target in &plan_a.target_endpoints {
            socket_a.send_to(payload_a.as_bytes(), target)?;
            sent_a += 1;
        }
        let payload_b = format!("{}:{}:{attempt}:{message}", room_id, peer_b);
        for target in &plan_b.target_endpoints {
            socket_b.send_to(payload_b.as_bytes(), target)?;
            sent_b += 1;
        }
        drain_udp_socket(&socket_a, &mut buffer, &mut received_by_a)?;
        drain_udp_socket(&socket_b, &mut buffer, &mut received_by_b)?;
        if interval_ms > 0 && attempt + 1 < attempts.max(1) {
            std::thread::sleep(Duration::from_millis(interval_ms));
        }
    }
    drain_udp_socket(&socket_a, &mut buffer, &mut received_by_a)?;
    drain_udp_socket(&socket_b, &mut buffer, &mut received_by_b)?;

    Ok(serde_json::json!({
        "status": if received_by_a > 0 && received_by_b > 0 { "ok" } else { "timeout" },
        "offerA": offer_a,
        "offerB": offer_b,
        "planA": plan_a,
        "planB": plan_b,
        "sentByA": sent_a,
        "sentByB": sent_b,
        "receivedByA": received_by_a,
        "receivedByB": received_by_b,
    }))
}

fn drain_udp_socket(
    socket: &UdpSocket,
    buffer: &mut [u8],
    received_count: &mut u16,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match socket.recv_from(buffer) {
            Ok((_, _)) => *received_count = received_count.saturating_add(1),
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
    Ok(())
}

fn drain_udp_socket_packet_records(
    socket: &UdpSocket,
    buffer: &mut [u8],
    received_packets: &mut Vec<serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match socket.recv_from(buffer) {
            Ok((bytes, peer)) => {
                received_packets.push(serde_json::json!({
                    "peer": peer.to_string(),
                    "bytes": bytes,
                    "text": String::from_utf8_lossy(&buffer[..bytes]).to_string(),
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
    Ok(())
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
    packet_io_probe_options: &RuntimePacketIoProbeOptions,
    wintun_runtime: bool,
    expected_broadcast_ports: Vec<u16>,
    expected_game_ports: Vec<u16>,
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
    let mut heartbeat_packets = Vec::new();
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
                let heartbeat = serde_json::json!({
                    "room_id": plan.room_id,
                    "peer_id": plan.local_peer_id,
                    "virtual_ip": plan.local_virtual_ip,
                    "kind": "runtime-heartbeat",
                    "sequence": sequence,
                    "sent_at_ms": current_epoch_ms(),
                });
                let envelope = seal_tunnel_payload(
                    key,
                    "runtime-heartbeat",
                    sequence,
                    current_epoch_ms(),
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
                let tick = serde_json::json!({
                    "status": "running",
                    "startedAtMs": started_at_ms,
                    "updatedAtMs": current_epoch_ms(),
                    "actualTunnelEndpoint": tunnel_endpoint.to_string(),
                    "bytesSent": bytes_sent,
                    "bytesReceived": bytes_received,
                    "heartbeatPacketsSent": heartbeat_packets.len(),
                    "tunnelPacketCount": tunnel_packets.len(),
                    "packetCaptureCount": capture_summaries.len(),
                    "forwardedPacketCount": forwarded_packets.len(),
                    "injectedPacketCount": injected_packets.len(),
                    "wintunRuntimeReceivedPacketCount": wintun_runtime_received_packets.len(),
                    "wintunRuntimeSentPacketCount": wintun_runtime_sent_packets.len(),
                    "packetIoProbe": packet_io_probe.clone(),
                    "adapterWriteStatus": packet_io_probe["adapterWriteStatus"].clone(),
                    "adapterReadStatus": packet_io_probe["adapterReadStatus"].clone(),
                    "wintunRuntime": wintun_runtime_open.clone(),
                    "lastError": last_error.clone(),
                });
                write_json_file(path, &tick)?;
                snapshot_write_count += 1;
                next_snapshot_at = Some(now + Duration::from_millis(interval_ms));
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
                        last_peer_packet_at = Some(Instant::now());
                        peer_timed_out = false;
                        connected_peer_count = connected_peer_count.max(1);
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
                            wintun_runtime_received_packets.push(serde_json::json!({
                                "packetIndex": packet_index,
                                "packetBytes": packet.packet_bytes,
                                "sourceIp": udp_packet.source_ip,
                                "destinationIp": udp_packet.destination_ip,
                                "sourcePort": udp_packet.source_port,
                                "destinationPort": udp_packet.destination_port,
                                "payloadBytes": udp_packet.payload.len(),
                                "broadcast": udp_packet.broadcast,
                                "forwarded": !heartbeat_targets.is_empty(),
                            }));

                            for target in &heartbeat_targets {
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
    let network_report = evaluate_network_observations(network_snapshot_from_runtime(
        None,
        Some(tunnel_snapshot.clone()),
        &capture_summaries,
        if plan.peers.is_empty() {
            connected_peer_count
        } else {
            plan.peers.len() as u16
        },
        runtime_expected_broadcast_ports,
        runtime_expected_game_ports,
    ));
    let heartbeat_packets_sent = heartbeat_packets.len();
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
        "actualTunnelEndpoint": tunnel_endpoint.to_string(),
        "tunnelServiceSnapshot": tunnel_snapshot,
        "heartbeatTargets": heartbeat_targets.iter().map(SocketAddr::to_string).collect::<Vec<_>>(),
        "heartbeatPackets": heartbeat_packets,
        "heartbeatPacketsSent": heartbeat_packets_sent,
        "snapshotWriteCount": snapshot_write_count,
        "tunnelPackets": tunnel_packets,
        "forwardedPackets": forwarded_packets,
        "rawVirtualPackets": raw_virtual_packets,
        "wintunRuntime": {
            "enabled": wintun_runtime,
            "open": wintun_runtime_open,
            "close": wintun_runtime_close,
            "receivedPackets": wintun_runtime_received_packets,
            "sentPackets": wintun_runtime_sent_packets,
            "errors": wintun_runtime_errors,
        },
        "injectedPackets": injected_packets,
        "injectedReceivedPackets": injected_received_packets,
        "injectTarget": inject_target.map(|target| target.to_string()),
        "packetCaptureSummaries": capture_summaries,
        "packetObservationLines": observation_lines,
        "networkObservation": network_report,
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
