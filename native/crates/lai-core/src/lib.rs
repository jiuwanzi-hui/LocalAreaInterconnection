pub mod broadcast_policy;
pub mod command_execution;
pub mod connection_path;
pub mod coordination_room_view;
pub mod coordination_store;
pub mod diagnostic_export;
pub mod diagnostics;
pub mod encrypted_tunnel;
pub mod firewall_diagnostics;
pub mod firewall_plan;
pub mod game_network_plan;
pub mod game_profile;
pub mod game_profile_catalog;
pub mod game_readiness;
pub mod invite;
pub mod ip;
pub mod join_plan;
pub mod nat_traversal;
pub mod network_observation;
pub mod p2p_handshake;
pub mod packet_observation_parser;
pub mod relay_fallback_plan;
pub mod room;
pub mod room_lifecycle;
pub mod room_runtime_plan;
pub mod runtime_cleanup_plan;
pub mod runtime_observation;
pub mod udp_forwarding;
pub mod virtual_adapter_ensure;
pub mod virtual_adapter_plan;
pub mod virtual_packet_io;
pub mod windows_adapter_parser;
pub mod windows_firewall_parser;
pub mod windows_netstat_parser;
pub mod windows_ping_parser;
pub mod windows_route_parser;
pub mod wintun_adapter;
pub mod wintun_adapter_delete;
pub mod wintun_adapter_open;
pub mod wintun_detect;
pub mod wintun_packet_receive;
pub mod wintun_packet_send;
pub mod wintun_runtime;
pub mod wintun_session;

pub use broadcast_policy::{
    create_broadcast_forward_report, should_forward_broadcast, BroadcastDecision,
    BroadcastForwardEvent, BroadcastForwardGate, BroadcastForwardReport, BroadcastPacket,
    BroadcastPolicy,
};
pub use command_execution::{
    create_command_execution_preview, CommandExecutionPreview, CommandExecutionRecord,
    CommandExecutionStatus,
};
pub use connection_path::{evaluate_connection_path, ConnectionPathReport};
pub use coordination_room_view::{
    coordination_room_view, CoordinationRoomMemberView, CoordinationRoomView,
};
pub use coordination_store::{
    close_coordination_room, close_coordination_room_by_peer, create_coordination_store,
    fetch_coordination_offers, heartbeat_coordination_peer, kick_coordination_peer,
    leave_coordination_room, prune_expired_coordination_peers, publish_coordination_offer,
    CoordinationCloseReport, CoordinationFetchResult, CoordinationKickReport,
    CoordinationLeaveReport, CoordinationPeer, CoordinationPruneReport, CoordinationRoom,
    CoordinationStore, CoordinationStoreUpdate,
};
pub use diagnostic_export::{
    create_diagnostic_export_bundle, DiagnosticAdapterScanSection, DiagnosticConnectionPathSection,
    DiagnosticExportBundle, DiagnosticExportEnvironment, DiagnosticExportInputs,
    DiagnosticExportSources, DiagnosticFirewallScanSection, DiagnosticGamePortScanSection,
    DiagnosticPacketSection, DiagnosticPingSection, DiagnosticRelayFallbackSection,
    DiagnosticRouteScanSection, DiagnosticRuntimeCleanupSection, DiagnosticRuntimePeerSummary,
    DiagnosticRuntimePeersSection, DiagnosticTextSource,
};
pub use diagnostics::{
    evaluate_diagnostics, DiagnosticProblem, DiagnosticReport, DiagnosticSnapshot,
};
pub use encrypted_tunnel::{
    open_tunnel_payload, seal_tunnel_payload, TunnelEnvelope, TunnelEnvelopeMetadata, TunnelPayload,
};
pub use firewall_diagnostics::{
    evaluate_firewall_diagnostics, observation_from_expected_rule, FirewallDiagnosticsReport,
    FirewallRuleCheck, FirewallRuleObservation,
};
pub use firewall_plan::{
    create_windows_firewall_plan, FirewallCommand, FirewallWarning, WindowsFirewallPlan,
};
pub use game_network_plan::{
    create_firewall_rules, create_game_network_plan, BroadcastPlan, FirewallRule, GameNetworkPlan,
    PlanWarning,
};
pub use game_profile::{
    normalize_ports, recommended_join_instruction, CompatibilityLevel, DiscoveryMode, GameProfile,
};
pub use game_profile_catalog::{
    find_game_profile, list_game_profile_summaries, parse_game_profile_catalog_json,
    profile_summary, GameProfileCatalog, GameProfileMatch, GameProfileSummary,
};
pub use game_readiness::{
    evaluate_game_readiness, evaluate_game_readiness_with_firewall,
    evaluate_game_readiness_with_firewall_and_connection_path, GameReadinessCheck,
    GameReadinessReport,
};
pub use invite::{create_invite, decode_invite, verify_invite, InvitePayload};
pub use ip::{broadcast_address, host_address, peer_address, subnet_for_room, Ipv4Subnet};
pub use join_plan::{create_join_plan, JoinPlan};
pub use nat_traversal::{
    create_coordination_message, create_nat_punch_plan, create_nat_traversal_offer,
    CoordinationMessage, NatCandidate, NatPunchPlan, NatTraversalOffer,
};
pub use network_observation::{
    evaluate_network_observations, AdapterObservation, NetworkObservationCheck,
    NetworkObservationReport, NetworkObservationSnapshot, PacketObservation,
    RuntimePeerObservation, TunnelObservation,
};
pub use p2p_handshake::{
    create_p2p_handshake_ack, create_p2p_handshake_hello, P2pHandshakeAck, P2pHandshakeHello,
};
pub use packet_observation_parser::{
    parse_packet_observation_line, parse_packet_observation_lines,
};
pub use relay_fallback_plan::{create_relay_fallback_plan, RelayFallbackPlan};
pub use room::{create_room, Room};
pub use room_lifecycle::{
    add_room_member, close_room, create_room_session, mark_member_left, update_member_connection,
    ConnectionPath, RoomLifecycleStatus, RoomMember, RoomMemberRole, RoomMemberStatus, RoomSession,
    RoomSessionSummary,
};
pub use room_runtime_plan::{
    create_room_runtime_plan, RoomRuntimePeer, RoomRuntimePlan, RuntimePortBinding,
    RuntimeTunnelPlan, RuntimeUdpForwardPlan,
};
pub use runtime_cleanup_plan::{
    create_runtime_cleanup_report, create_windows_runtime_cleanup_plan,
    create_windows_runtime_cleanup_plan_with_routes, RuntimeCleanupCheck, RuntimeCleanupPlan,
    RuntimeCleanupReport, RuntimeCleanupStep, RuntimeCleanupWarning,
};
pub use runtime_observation::{
    network_snapshot_from_runtime, network_snapshot_from_runtime_with_peers,
    packet_observation_from_capture_summary, tunnel_observation_from_service, PacketCaptureSummary,
    TunnelServiceSnapshot,
};
pub use udp_forwarding::{
    packet_observation_line_from_transport, packet_observation_line_from_udp_forward,
    udp_forward_summary, UdpForwardObservation, UdpForwardSummary,
};
pub use virtual_adapter_ensure::{
    create_windows_virtual_adapter_ensure_report, VirtualAdapterEnsureCheck,
    VirtualAdapterEnsureReport,
};
pub use virtual_adapter_plan::{
    create_windows_virtual_adapter_plan, AdapterPlanWarning, NetworkCommand, VirtualAdapterPlan,
};
pub use virtual_packet_io::{
    build_ipv4_tcp_packet, build_ipv4_udp_packet, create_virtual_packet_io_plan,
    parse_ipv4_packet_summary, parse_ipv4_tcp_packet, parse_ipv4_udp_packet,
    tcp_observation_from_virtual_packet, udp_observation_from_virtual_packet,
    VirtualIpv4PacketSummary, VirtualPacketIoPlan, VirtualTcpPacket, VirtualUdpPacket,
};
pub use windows_adapter_parser::parse_netsh_adapter_observation;
pub use windows_firewall_parser::parse_netsh_firewall_rules;
pub use windows_netstat_parser::{parse_windows_netstat_ano, WindowsNetstatEndpoint};
pub use windows_ping_parser::parse_windows_ping_observation;
pub use windows_route_parser::{parse_windows_ipv4_routes, WindowsRouteObservation};
pub use wintun_adapter::{
    create_wintun_adapter, WintunAdapterCreateReport, WintunAdapterCreateRequest,
};
pub use wintun_adapter_delete::{
    delete_wintun_adapter, WintunAdapterDeleteReport, WintunAdapterDeleteRequest,
};
pub use wintun_adapter_open::{
    open_wintun_adapter, WintunAdapterOpenReport, WintunAdapterOpenRequest,
};
pub use wintun_detect::{detect_wintun_availability, WintunDetectReport};
pub use wintun_packet_receive::{
    probe_wintun_packet_receive, WintunPacketReceiveProbeReport, WintunPacketReceiveProbeRequest,
    WintunReceivedPacketSummary,
};
pub use wintun_packet_send::{
    probe_wintun_packet_send, WintunPacketSendProbeReport, WintunPacketSendProbeRequest,
};
pub use wintun_runtime::{
    open_wintun_packet_io_session, validate_wintun_ring_capacity, WintunPacketIoCloseReport,
    WintunPacketIoConfig, WintunPacketIoOpenReport, WintunPacketIoSession, WintunRuntimePacket,
};
pub use wintun_session::{
    probe_wintun_session, WintunSessionProbeReport, WintunSessionProbeRequest,
};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("invalid ipv4 address: {0}")]
    InvalidIpv4(String),
    #[error("invalid cidr: {0}")]
    InvalidCidr(String),
    #[error("invalid invite code")]
    InvalidInvite,
    #[error("unsupported invite version: {0}")]
    UnsupportedInviteVersion(u16),
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error("crypto failed")]
    Crypto,
    #[error("invalid room state: {0}")]
    InvalidRoomState(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
