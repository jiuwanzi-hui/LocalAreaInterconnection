use crate::game_profile::{CompatibilityLevel, DiscoveryMode, GameProfile};
use crate::{CoreError, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameProfileCatalog {
    pub profiles: Vec<GameProfile>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameProfileMatch {
    pub profile: GameProfile,
    pub matched_by: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameProfileSummary {
    pub game_name: String,
    pub steam_app_id: Option<String>,
    pub discovery: DiscoveryMode,
    pub compatibility: CompatibilityLevel,
    pub port_count: usize,
    pub ports: Vec<u16>,
    pub join_method: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum GameProfileCatalogInput {
    Wrapped {
        profiles: Vec<GameProfileCatalogEntry>,
    },
    Profiles(Vec<GameProfileCatalogEntry>),
}

#[derive(Debug, Deserialize)]
struct GameProfileCatalogEntry {
    game_name: String,
    #[serde(default)]
    steam_app_id: Option<String>,
    #[serde(default = "default_discovery")]
    discovery: String,
    #[serde(default)]
    ports: Vec<u16>,
    #[serde(default = "default_join_method")]
    join_method: String,
    #[serde(default = "default_compatibility")]
    compatibility: String,
    #[serde(default)]
    notes: String,
}

pub fn parse_game_profile_catalog_json(text: &str) -> Result<GameProfileCatalog> {
    let input =
        serde_json::from_str::<GameProfileCatalogInput>(text.trim_start_matches('\u{feff}'))
            .map_err(|err| CoreError::Serialization(err.to_string()))?;
    let entries = match input {
        GameProfileCatalogInput::Wrapped { profiles } => profiles,
        GameProfileCatalogInput::Profiles(profiles) => profiles,
    };

    let profiles = entries
        .into_iter()
        .map(GameProfile::try_from)
        .collect::<Result<Vec<_>>>()?;
    Ok(GameProfileCatalog { profiles })
}

pub fn find_game_profile(
    catalog: &GameProfileCatalog,
    game_name: Option<&str>,
    steam_app_id: Option<&str>,
) -> Option<GameProfileMatch> {
    let steam_app_id = steam_app_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(steam_app_id) = steam_app_id {
        if let Some(profile) = catalog.profiles.iter().find(|profile| {
            profile
                .steam_app_id
                .as_deref()
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(steam_app_id))
        }) {
            return Some(GameProfileMatch {
                profile: profile.clone(),
                matched_by: "steam_app_id".to_owned(),
            });
        }
    }

    let game_name = game_name.map(str::trim).filter(|value| !value.is_empty())?;
    if let Some(profile) = catalog
        .profiles
        .iter()
        .find(|profile| profile.game_name.eq_ignore_ascii_case(game_name))
    {
        return Some(GameProfileMatch {
            profile: profile.clone(),
            matched_by: "game_name".to_owned(),
        });
    }

    catalog
        .profiles
        .iter()
        .find(|profile| {
            profile
                .game_name
                .to_lowercase()
                .contains(&game_name.to_lowercase())
        })
        .map(|profile| GameProfileMatch {
            profile: profile.clone(),
            matched_by: "game_name_contains".to_owned(),
        })
}

pub fn list_game_profile_summaries(
    catalog: &GameProfileCatalog,
    query: Option<&str>,
) -> Vec<GameProfileSummary> {
    let query = query
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    let mut profiles = catalog
        .profiles
        .iter()
        .filter(|profile| match &query {
            Some(query) => {
                profile.game_name.to_lowercase().contains(query)
                    || profile
                        .steam_app_id
                        .as_deref()
                        .is_some_and(|steam_app_id| steam_app_id.to_lowercase().contains(query))
            }
            None => true,
        })
        .map(profile_summary)
        .collect::<Vec<_>>();
    profiles.sort_by(|left, right| {
        left.game_name
            .to_lowercase()
            .cmp(&right.game_name.to_lowercase())
            .then_with(|| left.steam_app_id.cmp(&right.steam_app_id))
    });
    profiles
}

pub fn profile_summary(profile: &GameProfile) -> GameProfileSummary {
    let mut ports = profile.ports.clone();
    ports.sort_unstable();
    ports.dedup();
    GameProfileSummary {
        game_name: profile.game_name.clone(),
        steam_app_id: profile.steam_app_id.clone(),
        discovery: profile.discovery.clone(),
        compatibility: profile.compatibility.clone(),
        port_count: ports.len(),
        ports,
        join_method: profile.join_method.clone(),
    }
}

impl TryFrom<GameProfileCatalogEntry> for GameProfile {
    type Error = CoreError;

    fn try_from(entry: GameProfileCatalogEntry) -> Result<Self> {
        Ok(Self {
            game_name: entry.game_name,
            steam_app_id: entry.steam_app_id,
            discovery: parse_discovery_mode(&entry.discovery)?,
            ports: entry.ports,
            join_method: entry.join_method,
            compatibility: parse_compatibility_level(&entry.compatibility)?,
            notes: entry.notes,
        })
    }
}

fn parse_discovery_mode(value: &str) -> Result<DiscoveryMode> {
    match value {
        "udp_broadcast" => Ok(DiscoveryMode::UdpBroadcast),
        "direct_ip" => Ok(DiscoveryMode::DirectIp),
        "manual_ports" => Ok(DiscoveryMode::ManualPorts),
        "unknown" => Ok(DiscoveryMode::Unknown),
        other => Err(CoreError::Serialization(format!(
            "unsupported discovery mode `{other}`"
        ))),
    }
}

fn parse_compatibility_level(value: &str) -> Result<CompatibilityLevel> {
    match value {
        "A" | "a" => Ok(CompatibilityLevel::A),
        "B" | "b" => Ok(CompatibilityLevel::B),
        "C" | "c" => Ok(CompatibilityLevel::C),
        "D" | "d" => Ok(CompatibilityLevel::D),
        "unknown" => Ok(CompatibilityLevel::Unknown),
        other => Err(CoreError::Serialization(format!(
            "unsupported compatibility level `{other}`"
        ))),
    }
}

fn default_discovery() -> String {
    "unknown".to_owned()
}

fn default_join_method() -> String {
    "lan_list_or_direct_ip".to_owned()
}

fn default_compatibility() -> String {
    "unknown".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_parses_wrapped_profiles_and_matches_by_name() {
        let catalog = parse_game_profile_catalog_json(
            r#"{
                "profiles": [
                    {
                        "game_name": "Example Game",
                        "steam_app_id": "123456",
                        "discovery": "udp_broadcast",
                        "ports": [27016, 27015, 27015],
                        "join_method": "lan_list_or_direct_ip",
                        "compatibility": "A",
                        "notes": "Allow private network firewall access."
                    }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(catalog.profiles.len(), 1);
        assert_eq!(catalog.profiles[0].ports.len(), 3);

        let matched = find_game_profile(&catalog, Some("example game"), None).unwrap();
        assert_eq!(matched.matched_by, "game_name");
        assert_eq!(matched.profile.steam_app_id.as_deref(), Some("123456"));
        assert_eq!(matched.profile.discovery, DiscoveryMode::UdpBroadcast);
    }

    #[test]
    fn catalog_parses_raw_array_and_matches_by_steam_app_id() {
        let catalog = parse_game_profile_catalog_json(
            r#"[
                {
                    "game_name": "Direct Game",
                    "steam_app_id": "777",
                    "discovery": "direct_ip",
                    "ports": [7777],
                    "compatibility": "B"
                }
            ]"#,
        )
        .unwrap();

        let matched = find_game_profile(&catalog, Some("missing"), Some("777")).unwrap();
        assert_eq!(matched.matched_by, "steam_app_id");
        assert_eq!(matched.profile.join_method, "lan_list_or_direct_ip");
        assert_eq!(matched.profile.compatibility, CompatibilityLevel::B);
    }

    #[test]
    fn catalog_lists_sorted_summaries_and_filters_query() {
        let catalog = parse_game_profile_catalog_json(
            r#"{
                "profiles": [
                    {
                        "game_name": "Zeta Direct",
                        "steam_app_id": "200",
                        "discovery": "direct_ip",
                        "ports": [7777, 7777],
                        "compatibility": "B"
                    },
                    {
                        "game_name": "Alpha Broadcast",
                        "steam_app_id": "100",
                        "discovery": "udp_broadcast",
                        "ports": [27016, 27015],
                        "compatibility": "A"
                    }
                ]
            }"#,
        )
        .unwrap();

        let all = list_game_profile_summaries(&catalog, None);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].game_name, "Alpha Broadcast");
        assert_eq!(all[0].ports, vec![27015, 27016]);

        let filtered = list_game_profile_summaries(&catalog, Some("200"));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].game_name, "Zeta Direct");
        assert_eq!(filtered[0].port_count, 1);
    }
}
