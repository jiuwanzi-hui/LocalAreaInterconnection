use crate::ip::Ipv4Subnet;
use crate::network_observation::AdapterObservation;
use std::net::Ipv4Addr;

pub fn parse_netsh_adapter_observation(
    adapter_name: impl Into<String>,
    text: &str,
    expected_ip: Option<Ipv4Addr>,
    expected_subnet: Option<Ipv4Subnet>,
) -> Option<AdapterObservation> {
    if text.trim().is_empty() || !looks_like_adapter_config(text) {
        return None;
    }

    let assigned_ip =
        parse_value(text, &["ipaddress", "ipaddress(es)"]).and_then(|value| parse_ipv4(value));
    let parsed_subnet = parse_value(text, &["subnetprefix", "subnetmask"]).and_then(parse_subnet);

    Some(AdapterObservation {
        adapter_name: adapter_name.into(),
        enabled: !contains_disabled_marker(text),
        expected_ip,
        assigned_ip,
        virtual_subnet: expected_subnet.or(parsed_subnet),
        mtu: parse_value(text, &["mtu"]).and_then(parse_u16),
        interface_metric: parse_value(text, &["interfacemetric", "interface metric"])
            .and_then(parse_u16),
    })
}

fn looks_like_adapter_config(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim().to_ascii_lowercase();
        line.starts_with("configuration for interface")
            || normalize_key(&line).contains("ipaddress")
    })
}

fn contains_disabled_marker(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    text.contains("disabled") || text.contains("not enabled")
}

fn parse_value<'a>(text: &'a str, keys: &[&str]) -> Option<&'a str> {
    text.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        let key = normalize_key(key);
        if keys.iter().any(|expected| key == normalize_key(expected)) {
            Some(value.trim())
        } else {
            None
        }
    })
}

fn parse_ipv4(value: &str) -> Option<Ipv4Addr> {
    value
        .split(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .find_map(|item| item.parse::<Ipv4Addr>().ok())
}

fn parse_subnet(value: &str) -> Option<Ipv4Subnet> {
    value.split_whitespace().find_map(|item| {
        item.trim_matches(|ch| ch == '(' || ch == ')')
            .parse::<Ipv4Subnet>()
            .ok()
    })
}

fn parse_u16(value: &str) -> Option<u16> {
    value
        .split(|ch: char| !ch.is_ascii_digit())
        .find(|item| !item.is_empty())
        .and_then(|item| item.parse::<u16>().ok())
}

fn normalize_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_english_netsh_adapter_config() {
        let text = r#"
Configuration for interface "LocalAreaInterconnection"
    DHCP enabled:                         No
    IP Address:                           10.77.12.2
    Subnet Prefix:                        10.77.12.0/24 (mask 255.255.255.0)
    InterfaceMetric:                      5
"#;

        let observation = parse_netsh_adapter_observation(
            "LocalAreaInterconnection",
            text,
            Some("10.77.12.2".parse().unwrap()),
            None,
        )
        .unwrap();

        assert!(observation.enabled);
        assert_eq!(observation.assigned_ip, Some("10.77.12.2".parse().unwrap()));
        assert_eq!(
            observation.virtual_subnet.unwrap().to_string(),
            "10.77.12.0/24"
        );
        assert_eq!(observation.interface_metric, Some(5));
    }

    #[test]
    fn empty_adapter_output_returns_none() {
        assert!(parse_netsh_adapter_observation("missing", "", None, None).is_none());
    }
}
