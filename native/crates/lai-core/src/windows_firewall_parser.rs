use crate::firewall_diagnostics::FirewallRuleObservation;
use std::collections::BTreeMap;

pub fn parse_netsh_firewall_rules(output: &str) -> Vec<FirewallRuleObservation> {
    let mut blocks = Vec::new();
    let mut current = BTreeMap::new();

    for line in output.lines() {
        let Some((key, value)) = parse_key_value(line) else {
            continue;
        };
        if key == "rulename" && !current.is_empty() {
            blocks.push(current);
            current = BTreeMap::new();
        }
        current.insert(key, value);
    }
    if !current.is_empty() {
        blocks.push(current);
    }

    blocks
        .iter()
        .flat_map(observations_from_block)
        .collect::<Vec<_>>()
}

fn observations_from_block(block: &BTreeMap<String, String>) -> Vec<FirewallRuleObservation> {
    let protocol = normalize_protocol(block.get("protocol").map(String::as_str));
    if protocol.is_empty() {
        return Vec::new();
    }
    parse_ports(block.get("localport").map(String::as_str))
        .into_iter()
        .map(|port| FirewallRuleObservation {
            rule_name: block.get("rulename").cloned(),
            direction: normalize_direction(block.get("direction").map(String::as_str)),
            action: normalize_action(block.get("action").map(String::as_str)),
            protocol: protocol.clone(),
            port,
            profile: normalize_profile(block.get("profiles").map(String::as_str)),
            remote_scope: block
                .get("remoteip")
                .map(String::as_str)
                .filter(|value| !value.eq_ignore_ascii_case("any"))
                .map(str::to_owned),
            program: block
                .get("program")
                .map(String::as_str)
                .filter(|value| !value.eq_ignore_ascii_case("any"))
                .map(str::to_owned),
            enabled: normalize_enabled(block.get("enabled").map(String::as_str)),
        })
        .collect()
}

fn parse_key_value(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once(':')?;
    let key = key
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    let value = value.trim().to_owned();
    if key.is_empty() {
        None
    } else {
        Some((key, value))
    }
}

fn parse_ports(value: Option<&str>) -> Vec<u16> {
    let Some(value) = value else {
        return Vec::new();
    };
    if value.eq_ignore_ascii_case("any") {
        return Vec::new();
    }
    value
        .split(',')
        .filter_map(|item| item.trim().parse::<u16>().ok())
        .collect()
}

fn normalize_protocol(value: Option<&str>) -> String {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "udp" => "udp".to_owned(),
        "tcp" => "tcp".to_owned(),
        _ => String::new(),
    }
}

fn normalize_direction(value: Option<&str>) -> String {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "in" | "inbound" => "inbound".to_owned(),
        "out" | "outbound" => "outbound".to_owned(),
        _ => "unknown".to_owned(),
    }
}

fn normalize_action(value: Option<&str>) -> String {
    match value
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "allow" => "allow".to_owned(),
        "block" => "block".to_owned(),
        _ => "unknown".to_owned(),
    }
}

fn normalize_profile(value: Option<&str>) -> String {
    let value = value.unwrap_or_default().trim().to_ascii_lowercase();
    if value.contains("private") {
        "private".to_owned()
    } else if value.contains("domain") {
        "domain".to_owned()
    } else if value.contains("public") {
        "public".to_owned()
    } else {
        "unknown".to_owned()
    }
}

fn normalize_enabled(value: Option<&str>) -> bool {
    matches!(
        value
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "yes" | "true" | "enabled"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_netsh_firewall_rules_into_observations() {
        let output = r#"
Rule Name:                            Example UDP 7777
----------------------------------------------------------------------
Enabled:                              Yes
Direction:                            In
Profiles:                             Private
Grouping:                             LocalAreaInterconnection
LocalIP:                              Any
RemoteIP:                             10.77.12.0/24
Protocol:                             UDP
LocalPort:                            7777
RemotePort:                           Any
Program:                              C:\Games\Example Game\game.exe
Action:                               Allow

Rule Name:                            Example TCP 7777
----------------------------------------------------------------------
Enabled:                              No
Direction:                            In
Profiles:                             Private
RemoteIP:                             Any
Protocol:                             TCP
LocalPort:                            7777
Action:                               Allow
"#;

        let observations = parse_netsh_firewall_rules(output);

        assert_eq!(observations.len(), 2);
        assert_eq!(
            observations[0].rule_name.as_deref(),
            Some("Example UDP 7777")
        );
        assert_eq!(observations[0].protocol, "udp");
        assert_eq!(observations[0].port, 7777);
        assert_eq!(observations[0].direction, "inbound");
        assert_eq!(observations[0].profile, "private");
        assert_eq!(
            observations[0].remote_scope.as_deref(),
            Some("10.77.12.0/24")
        );
        assert_eq!(
            observations[0].program.as_deref(),
            Some("C:\\Games\\Example Game\\game.exe")
        );
        assert!(observations[0].enabled);
        assert!(!observations[1].enabled);
    }

    #[test]
    fn skips_rules_without_concrete_tcp_or_udp_ports() {
        let output = r#"
Rule Name:                            Any Port Rule
Enabled:                              Yes
Direction:                            In
Profiles:                             Private
Protocol:                             Any
LocalPort:                            Any
Action:                               Allow
"#;

        assert!(parse_netsh_firewall_rules(output).is_empty());
    }
}
