use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum DiscoveryMode {
    UdpBroadcast,
    DirectIp,
    ManualPorts,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum CompatibilityLevel {
    A,
    B,
    C,
    D,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameProfile {
    pub game_name: String,
    pub steam_app_id: Option<String>,
    pub discovery: DiscoveryMode,
    pub ports: Vec<u16>,
    pub join_method: String,
    pub compatibility: CompatibilityLevel,
    pub notes: String,
}

pub fn normalize_ports(ports: &[u16]) -> Vec<u16> {
    let mut ports = ports.to_vec();
    ports.sort_unstable();
    ports.dedup();
    ports
}

pub fn recommended_join_instruction(profile: &GameProfile) -> &'static str {
    match (&profile.discovery, &profile.compatibility) {
        (DiscoveryMode::UdpBroadcast, CompatibilityLevel::A) => "Open the game LAN list; if it is empty, try Direct IP.",
        (DiscoveryMode::DirectIp, _) | (_, CompatibilityLevel::B) => "Copy the host virtual IP and join with Direct IP.",
        (DiscoveryMode::ManualPorts, _) | (_, CompatibilityLevel::C) => "Add game port rules before trying LAN list or Direct IP.",
        (_, CompatibilityLevel::D) => "This game is not a good MVP target because platform or game behavior may block virtual LAN.",
        _ => "Try the LAN list first, then inspect broadcast, ports, and Direct IP.",
    }
}
