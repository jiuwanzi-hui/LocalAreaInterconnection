use crate::broadcast_policy::BroadcastPolicy;
use crate::game_profile::{
    normalize_ports, recommended_join_instruction, CompatibilityLevel, DiscoveryMode, GameProfile,
};
use crate::ip::{broadcast_address, Ipv4Subnet};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameNetworkPlan {
    pub game_name: String,
    pub compatibility: CompatibilityLevel,
    pub virtual_subnet: String,
    pub host_ip: Option<Ipv4Addr>,
    pub local_ip: Option<Ipv4Addr>,
    pub broadcast_address: Ipv4Addr,
    pub join_instruction: String,
    pub firewall_rules: Vec<FirewallRule>,
    pub broadcast: BroadcastPlan,
    pub diagnostic_checks: Vec<String>,
    pub warnings: Vec<PlanWarning>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FirewallRule {
    pub name: String,
    pub direction: String,
    pub action: String,
    pub protocol: String,
    pub port: u16,
    pub profile: String,
    pub remote_scope: String,
    pub purpose: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BroadcastPlan {
    pub enabled: bool,
    pub policy: Option<BroadcastPolicy>,
    pub expectation: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlanWarning {
    pub key: String,
    pub message: String,
}

pub fn create_game_network_plan(
    profile: &GameProfile,
    virtual_subnet: Ipv4Subnet,
    host_ip: Option<Ipv4Addr>,
    local_ip: Option<Ipv4Addr>,
    max_broadcast_packets_per_second: u16,
) -> GameNetworkPlan {
    let ports = normalize_ports(&profile.ports);
    let broadcast_enabled = !matches!(&profile.discovery, DiscoveryMode::DirectIp)
        && !matches!(&profile.compatibility, CompatibilityLevel::D);
    let broadcast_policy = if broadcast_enabled {
        Some(BroadcastPolicy::with_limit(
            virtual_subnet,
            ports.clone(),
            max_broadcast_packets_per_second,
        ))
    } else {
        None
    };

    GameNetworkPlan {
        game_name: profile.game_name.clone(),
        compatibility: profile.compatibility.clone(),
        virtual_subnet: virtual_subnet.to_string(),
        host_ip,
        local_ip,
        broadcast_address: broadcast_address(virtual_subnet),
        join_instruction: recommended_join_instruction(profile).to_owned(),
        firewall_rules: create_firewall_rules(profile, virtual_subnet),
        broadcast: BroadcastPlan {
            enabled: broadcast_enabled,
            policy: broadcast_policy,
            expectation: if broadcast_enabled {
                "Capture UDP broadcasts on the virtual subnet and forward them to room members."
                    .to_owned()
            } else {
                "This profile does not depend on LAN broadcast discovery.".to_owned()
            },
        },
        diagnostic_checks: create_diagnostic_checks(profile, broadcast_enabled),
        warnings: create_plan_warnings(profile, &ports, broadcast_enabled),
    }
}

pub fn create_firewall_rules(
    profile: &GameProfile,
    virtual_subnet: Ipv4Subnet,
) -> Vec<FirewallRule> {
    let mut rules = Vec::new();
    for port in normalize_ports(&profile.ports) {
        for protocol in ["udp", "tcp"] {
            rules.push(FirewallRule {
                name: format!("{} {} {}", profile.game_name, protocol.to_uppercase(), port),
                direction: "inbound".to_owned(),
                action: "allow".to_owned(),
                protocol: protocol.to_owned(),
                port,
                profile: "private".to_owned(),
                remote_scope: virtual_subnet.to_string(),
                purpose: "Allow room members to reach the game port through the virtual subnet."
                    .to_owned(),
            });
        }
    }
    rules
}

fn create_diagnostic_checks(profile: &GameProfile, broadcast_enabled: bool) -> Vec<String> {
    let mut checks = vec![
        "virtual-adapter".to_owned(),
        "tunnel".to_owned(),
        "p2p".to_owned(),
        "direct-ip".to_owned(),
    ];
    if !profile.ports.is_empty() {
        checks.push("firewall".to_owned());
        checks.push("game-traffic".to_owned());
    }
    if broadcast_enabled {
        checks.push("broadcast".to_owned());
    }
    checks.sort();
    checks.dedup();
    checks
}

fn create_plan_warnings(
    profile: &GameProfile,
    ports: &[u16],
    broadcast_enabled: bool,
) -> Vec<PlanWarning> {
    let mut warnings = Vec::new();
    if ports.is_empty() {
        warnings.push(PlanWarning {
            key: "unknown-ports".to_owned(),
            message: "This game profile has no port data; firewall rules need manual follow-up."
                .to_owned(),
        });
    }
    if !broadcast_enabled && matches!(&profile.discovery, DiscoveryMode::DirectIp) {
        warnings.push(PlanWarning {
            key: "direct-ip-only".to_owned(),
            message:
                "This profile prefers Direct IP; LAN room discovery may not work automatically."
                    .to_owned(),
        });
    }
    if matches!(&profile.compatibility, CompatibilityLevel::D) {
        warnings.push(PlanWarning {
            key: "poor-mvp-target".to_owned(),
            message: "This game is a poor MVP target because platform or game behavior may block virtual LAN.".to_owned(),
        });
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_network_plan_creates_firewall_and_broadcast_rules() {
        let profile = GameProfile {
            game_name: "Example".to_owned(),
            steam_app_id: None,
            discovery: DiscoveryMode::UdpBroadcast,
            ports: vec![27016, 27015, 27015],
            join_method: "lan_list_or_direct_ip".to_owned(),
            compatibility: CompatibilityLevel::A,
            notes: String::new(),
        };
        let subnet = "10.77.12.0/24".parse::<Ipv4Subnet>().unwrap();
        let plan = create_game_network_plan(
            &profile,
            subnet,
            Some("10.77.12.1".parse().unwrap()),
            Some("10.77.12.2".parse().unwrap()),
            30,
        );

        assert!(plan.broadcast.enabled);
        assert_eq!(
            plan.broadcast_address,
            "10.77.12.255".parse::<Ipv4Addr>().unwrap()
        );
        assert_eq!(plan.firewall_rules.len(), 4);
        assert_eq!(plan.firewall_rules[0].remote_scope, "10.77.12.0/24");
        assert!(plan.diagnostic_checks.contains(&"broadcast".to_owned()));
        assert!(plan.diagnostic_checks.contains(&"game-traffic".to_owned()));
    }

    #[test]
    fn direct_ip_plan_disables_broadcast() {
        let profile = GameProfile {
            game_name: "Direct Only".to_owned(),
            steam_app_id: None,
            discovery: DiscoveryMode::DirectIp,
            ports: vec![7777],
            join_method: "direct_ip".to_owned(),
            compatibility: CompatibilityLevel::B,
            notes: String::new(),
        };
        let subnet = "10.77.44.0/24".parse::<Ipv4Subnet>().unwrap();
        let plan = create_game_network_plan(&profile, subnet, None, None, 30);

        assert!(!plan.broadcast.enabled);
        assert!(plan.broadcast.policy.is_none());
        assert!(plan.diagnostic_checks.contains(&"direct-ip".to_owned()));
        assert!(!plan.diagnostic_checks.contains(&"broadcast".to_owned()));
        assert!(plan
            .warnings
            .iter()
            .any(|warning| warning.key == "direct-ip-only"));
    }
}
