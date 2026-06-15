use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lai-cli")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Subcommand)]
pub(crate) enum Command {
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
    RuntimeCleanupPlan {
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        peer_id: String,
        #[arg(long)]
        virtual_ip: String,
        #[arg(long)]
        subnet: Option<String>,
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long, default_value = "userspace-udp")]
        packet_io_backend: String,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        restore_adapter: bool,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        cleanup_routes: bool,
    },
    RuntimeCleanupReport {
        #[arg(long)]
        runtime_snapshot: Option<String>,
        #[arg(long)]
        cleanup_plan: Option<String>,
        #[arg(long)]
        adapter_netsh_output: Option<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        adapter_scan: bool,
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long)]
        route_output: Option<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        route_scan: bool,
    },
    RuntimeCleanupApply {
        #[arg(long)]
        runtime_snapshot: Option<String>,
        #[arg(long)]
        cleanup_plan: Option<String>,
        #[arg(long)]
        adapter_netsh_output: Option<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        adapter_scan: bool,
        #[arg(long, default_value = "LocalAreaInterconnection")]
        adapter_name: String,
        #[arg(long)]
        route_output: Option<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        route_scan: bool,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        yes: bool,
    },
    RouteScan {
        #[arg(long)]
        route_output: Option<String>,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        route_scan: bool,
        #[arg(long)]
        virtual_ip: Option<String>,
        #[arg(long)]
        subnet: Option<String>,
    },
    GamePortScan {
        #[arg(long)]
        netstat_output: Option<String>,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        netstat_scan: bool,
        #[arg(long, default_value = "Generic LAN Game")]
        game_name: String,
        #[arg(long)]
        catalog: Option<String>,
        #[arg(long)]
        steam_app_id: Option<String>,
        #[arg(long, default_value = "")]
        ports: String,
        #[arg(long, default_value = "udp,tcp")]
        protocols: String,
    },
    GameReadiness {
        #[arg(long)]
        network_report: String,
        #[arg(long)]
        game_plan: Option<String>,
        #[arg(long)]
        catalog: Option<String>,
        #[arg(long, default_value = "Generic LAN Game")]
        game_name: String,
        #[arg(long)]
        steam_app_id: Option<String>,
        #[arg(long)]
        subnet: String,
        #[arg(long, default_value = "manual_ports")]
        discovery: String,
        #[arg(long, default_value = "")]
        ports: String,
        #[arg(long, default_value = "unknown")]
        compatibility: String,
        #[arg(long)]
        host_ip: Option<String>,
        #[arg(long)]
        local_ip: Option<String>,
        #[arg(long)]
        firewall_netsh_output: Option<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        firewall_scan: bool,
        #[arg(long)]
        program: Option<String>,
        #[arg(long)]
        netstat_output: Option<String>,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        netstat_scan: bool,
        #[arg(long, default_value = "udp,tcp")]
        protocols: String,
        #[arg(long)]
        relay_local_offer: Option<String>,
        #[arg(long)]
        relay_remote_offer: Option<String>,
        #[arg(long, default_value = "unknown")]
        relay_p2p_status: String,
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
        #[arg(long)]
        coordination_server: Option<String>,
        #[arg(long = "coordination-peer")]
        coordination_peers: Vec<String>,
        #[arg(long, default_value = "")]
        game_ports: String,
        #[arg(long, default_value = "")]
        broadcast_ports: String,
        #[arg(long, default_value_t = 30)]
        max_broadcast_packets_per_second: u16,
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
        nat_bootstrap_stun_server: Option<String>,
        #[arg(long, default_value_t = 1000)]
        nat_bootstrap_stun_timeout_ms: u64,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        nat_bootstrap_upnp_port_map: bool,
        #[arg(long, default_value_t = 1500)]
        nat_bootstrap_upnp_timeout_ms: u64,
        #[arg(long, default_value_t = 7200)]
        nat_bootstrap_upnp_lease_seconds: u32,
        #[arg(long, hide = true)]
        nat_bootstrap_upnp_gateway_location: Option<String>,
        #[arg(long)]
        stop_file: Option<String>,
        #[arg(long)]
        snapshot_interval_ms: Option<u64>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        coordination_monitor: bool,
        #[arg(long, default_value_t = 1000)]
        coordination_monitor_interval_ms: u64,
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
    GameProfilePlan {
        #[arg(long)]
        catalog: String,
        #[arg(long)]
        game_name: Option<String>,
        #[arg(long)]
        steam_app_id: Option<String>,
        #[arg(long)]
        subnet: String,
        #[arg(long)]
        host_ip: Option<String>,
        #[arg(long)]
        local_ip: Option<String>,
        #[arg(long, default_value_t = 30)]
        max_broadcast_packets_per_second: u16,
    },
    GameProfileList {
        #[arg(long)]
        catalog: String,
        #[arg(long)]
        query: Option<String>,
    },
    FirewallPlan {
        #[arg(long, default_value = "Generic LAN Game")]
        game_name: String,
        #[arg(long)]
        catalog: Option<String>,
        #[arg(long)]
        steam_app_id: Option<String>,
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
        catalog: Option<String>,
        #[arg(long)]
        steam_app_id: Option<String>,
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
    RelayUdpServer {
        #[arg(long, default_value = "0.0.0.0:39091")]
        bind: String,
        #[arg(long)]
        key: String,
        #[arg(long, default_value = "room_test")]
        room_id: String,
        #[arg(long = "allowed-peer")]
        allowed_peers: Vec<String>,
        #[arg(long, default_value_t = 0)]
        max_packets: u16,
        #[arg(long, default_value_t = 30000)]
        timeout_ms: u64,
    },
    RelayUdpLoopbackTest {
        #[arg(long, default_value = "127.0.0.1:0")]
        bind: String,
        #[arg(long)]
        key: String,
        #[arg(long, default_value = "room_test")]
        room_id: String,
        #[arg(long, default_value = "relay ping")]
        message: String,
        #[arg(long, default_value_t = 2000)]
        timeout_ms: u64,
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
        #[arg(long)]
        virtual_ip: Option<String>,
        #[arg(long, default_value = "0.0.0.0:0")]
        bind: String,
        #[arg(long)]
        observed_endpoint: Option<String>,
        #[arg(long)]
        stun_server: Option<String>,
        #[arg(long, default_value_t = 1000)]
        stun_timeout_ms: u64,
        #[arg(long = "relay")]
        relay_endpoints: Vec<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        upnp_port_map: bool,
        #[arg(long, default_value_t = 1500)]
        upnp_timeout_ms: u64,
        #[arg(long, default_value_t = 7200)]
        upnp_lease_seconds: u32,
        #[arg(long, hide = true)]
        upnp_gateway_location: Option<String>,
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
    RelayFallbackPlan {
        #[arg(long)]
        local_offer: String,
        #[arg(long)]
        remote_offer: String,
        #[arg(long, default_value = "unknown")]
        p2p_status: String,
    },
    ConnectionPathPlan {
        #[arg(long)]
        local_offer: String,
        #[arg(long)]
        remote_offer: String,
        #[arg(long, default_value = "unknown")]
        p2p_status: String,
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
        #[arg(long)]
        stun_server: Option<String>,
        #[arg(long, default_value_t = 1000)]
        stun_timeout_ms: u64,
        #[arg(long = "relay")]
        relay_endpoints: Vec<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        upnp_port_map: bool,
        #[arg(long, default_value_t = 1500)]
        upnp_timeout_ms: u64,
        #[arg(long, default_value_t = 7200)]
        upnp_lease_seconds: u32,
        #[arg(long, hide = true)]
        upnp_gateway_location: Option<String>,
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
        #[arg(long)]
        stun_server: Option<String>,
        #[arg(long, default_value_t = 1000)]
        stun_timeout_ms: u64,
        #[arg(long = "relay")]
        relay_endpoints: Vec<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        upnp_port_map: bool,
        #[arg(long, default_value_t = 1500)]
        upnp_timeout_ms: u64,
        #[arg(long, default_value_t = 7200)]
        upnp_lease_seconds: u32,
        #[arg(long, hide = true)]
        upnp_gateway_location: Option<String>,
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
    StunLikeServe {
        #[arg(long, default_value = "0.0.0.0:39120")]
        bind: String,
        #[arg(long, default_value_t = 0)]
        max_requests: u32,
        #[arg(long, default_value_t = 30000)]
        timeout_ms: u64,
    },
    StunLikeQuery {
        #[arg(long, default_value = "0.0.0.0:0")]
        bind: String,
        #[arg(long)]
        server: String,
        #[arg(long, default_value_t = 1000)]
        timeout_ms: u64,
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
    CoordinationLeave {
        #[arg(long)]
        store: String,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        peer_id: String,
    },
    CoordinationKick {
        #[arg(long)]
        store: String,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        peer_id: String,
        #[arg(long)]
        kicked_by: String,
    },
    CoordinationClose {
        #[arg(long)]
        store: String,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        closed_by: Option<String>,
    },
    CoordinationRoomView {
        #[arg(long)]
        store: String,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        peer_id: String,
        #[arg(long)]
        subnet: String,
    },
    CoordinationPrune {
        #[arg(long)]
        store: String,
    },
    CoordinationHttpServe {
        #[arg(long, default_value = "127.0.0.1:39110")]
        bind: String,
        #[arg(long)]
        store: String,
        #[arg(long, default_value_t = 0)]
        max_requests: u32,
        #[arg(long, default_value_t = 30000)]
        request_timeout_ms: u64,
    },
    CoordinationHttpOfferPublish {
        #[arg(long)]
        server: String,
        #[arg(long)]
        offer: String,
        #[arg(long, default_value_t = 30000)]
        ttl_ms: u128,
    },
    CoordinationHttpOfferFetch {
        #[arg(long)]
        server: String,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        peer_id: String,
    },
    CoordinationHttpRoomView {
        #[arg(long)]
        server: String,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        peer_id: String,
        #[arg(long)]
        subnet: String,
    },
    CoordinationHttpHeartbeat {
        #[arg(long)]
        server: String,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        peer_id: String,
        #[arg(long, default_value_t = 30000)]
        ttl_ms: u128,
    },
    CoordinationHttpLeave {
        #[arg(long)]
        server: String,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        peer_id: String,
    },
    CoordinationHttpKick {
        #[arg(long)]
        server: String,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        peer_id: String,
        #[arg(long)]
        kicked_by: String,
    },
    CoordinationHttpClose {
        #[arg(long)]
        server: String,
        #[arg(long)]
        room_id: String,
        #[arg(long)]
        closed_by: Option<String>,
    },
    CoordinationHttpPrune {
        #[arg(long)]
        server: String,
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
    UdpLoopbackTest {
        #[arg(long, default_value_t = 39077)]
        port: u16,
        #[arg(long, default_value = "ping")]
        message: String,
        #[arg(long, default_value_t = 3000)]
        timeout_ms: u64,
        #[arg(long)]
        observe_file: Option<String>,
    },
    UdpBroadcastTest {
        #[arg(long, default_value_t = 39078)]
        port: u16,
        #[arg(long, default_value = "discover")]
        message: String,
        #[arg(long, default_value_t = 3000)]
        timeout_ms: u64,
        #[arg(long)]
        observe_file: Option<String>,
    },
    TcpLoopbackTest {
        #[arg(long, default_value_t = 39079)]
        port: u16,
        #[arg(long, default_value = "ping")]
        message: String,
        #[arg(long, default_value_t = 3000)]
        timeout_ms: u64,
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
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        adapter_scan: bool,
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
        connection_path: Option<String>,
        #[arg(long)]
        ping_test: Option<String>,
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
        #[arg(long)]
        route_output: Option<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        route_scan: bool,
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
        #[arg(long)]
        runtime_snapshot: Option<String>,
        #[arg(long)]
        route_output: Option<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        route_scan: bool,
        #[arg(long)]
        netstat_output: Option<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
        netstat_scan: bool,
        #[arg(long, default_value = "Generic LAN Game")]
        game_name: String,
        #[arg(long)]
        catalog: Option<String>,
        #[arg(long)]
        steam_app_id: Option<String>,
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
        #[arg(long)]
        relay_local_offer: Option<String>,
        #[arg(long)]
        relay_remote_offer: Option<String>,
        #[arg(long, default_value = "failed")]
        relay_p2p_status: String,
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
