pub mod broadcast_policy;
pub mod diagnostic_export;
pub mod diagnostics;
pub mod firewall_diagnostics;
pub mod firewall_plan;
pub mod game_network_plan;
pub mod game_profile;
pub mod invite;
pub mod ip;
pub mod join_plan;
pub mod network_observation;
pub mod packet_observation_parser;
pub mod room;
pub mod room_lifecycle;
pub mod runtime_observation;
pub mod virtual_adapter_plan;
pub mod windows_adapter_parser;
pub mod windows_firewall_parser;
pub mod windows_ping_parser;

pub use broadcast_policy::{
    should_forward_broadcast, BroadcastDecision, BroadcastPacket, BroadcastPolicy,
};
pub use diagnostic_export::{
    create_diagnostic_export_bundle, DiagnosticAdapterScanSection, DiagnosticExportBundle,
    DiagnosticExportEnvironment, DiagnosticExportInputs, DiagnosticExportSources,
    DiagnosticFirewallScanSection, DiagnosticPacketSection, DiagnosticPingSection,
    DiagnosticTextSource,
};
pub use diagnostics::{
    evaluate_diagnostics, DiagnosticProblem, DiagnosticReport, DiagnosticSnapshot,
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
pub use invite::{create_invite, decode_invite, verify_invite, InvitePayload};
pub use ip::{broadcast_address, host_address, peer_address, subnet_for_room, Ipv4Subnet};
pub use join_plan::{create_join_plan, JoinPlan};
pub use network_observation::{
    evaluate_network_observations, AdapterObservation, NetworkObservationCheck,
    NetworkObservationReport, NetworkObservationSnapshot, PacketObservation, TunnelObservation,
};
pub use packet_observation_parser::{
    parse_packet_observation_line, parse_packet_observation_lines,
};
pub use room::{create_room, Room};
pub use room_lifecycle::{
    add_room_member, close_room, create_room_session, mark_member_left, update_member_connection,
    ConnectionPath, RoomLifecycleStatus, RoomMember, RoomMemberRole, RoomMemberStatus, RoomSession,
    RoomSessionSummary,
};
pub use runtime_observation::{
    network_snapshot_from_runtime, packet_observation_from_capture_summary,
    tunnel_observation_from_service, PacketCaptureSummary, TunnelServiceSnapshot,
};
pub use virtual_adapter_plan::{
    create_windows_virtual_adapter_plan, AdapterPlanWarning, NetworkCommand, VirtualAdapterPlan,
};
pub use windows_adapter_parser::parse_netsh_adapter_observation;
pub use windows_firewall_parser::parse_netsh_firewall_rules;
pub use windows_ping_parser::parse_windows_ping_observation;

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
