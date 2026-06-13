use crate::ip::Ipv4Subnet;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowsRouteObservation {
    pub destination: Ipv4Subnet,
    pub gateway: Option<Ipv4Addr>,
    pub interface_ip: Option<Ipv4Addr>,
    pub metric: Option<u32>,
    pub persistent: bool,
}

pub fn parse_windows_ipv4_routes(text: &str) -> Vec<WindowsRouteObservation> {
    let mut routes = Vec::new();
    let mut in_ipv4_table = false;
    let mut in_active_routes = false;
    let mut in_persistent_routes = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.contains("ipv4 route table") {
            in_ipv4_table = true;
            in_active_routes = false;
            in_persistent_routes = false;
            continue;
        }
        if in_ipv4_table && lower.starts_with("ipv6 route table") {
            break;
        }
        if !in_ipv4_table {
            continue;
        }
        if lower.starts_with("active routes") {
            in_active_routes = true;
            in_persistent_routes = false;
            continue;
        }
        if lower.starts_with("persistent routes") {
            in_active_routes = false;
            in_persistent_routes = true;
            continue;
        }
        if lower.starts_with("network destination")
            || lower.starts_with("network address")
            || lower.starts_with("none")
            || lower.starts_with("====")
            || lower.starts_with("---")
        {
            continue;
        }

        if in_active_routes {
            if let Some(route) = parse_active_route(trimmed) {
                routes.push(route);
            }
        } else if in_persistent_routes {
            if let Some(route) = parse_persistent_route(trimmed) {
                routes.push(route);
            }
        }
    }

    routes
}

fn parse_active_route(line: &str) -> Option<WindowsRouteObservation> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 5 {
        return None;
    }
    let destination = subnet_from_destination_and_mask(parts[0], parts[1])?;
    Some(WindowsRouteObservation {
        destination,
        gateway: parse_gateway(parts[2]),
        interface_ip: parts[3].parse::<Ipv4Addr>().ok(),
        metric: parts[4].parse::<u32>().ok(),
        persistent: false,
    })
}

fn parse_persistent_route(line: &str) -> Option<WindowsRouteObservation> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 4 {
        return None;
    }
    let destination = subnet_from_destination_and_mask(parts[0], parts[1])?;
    Some(WindowsRouteObservation {
        destination,
        gateway: parse_gateway(parts[2]),
        interface_ip: None,
        metric: parts[3].parse::<u32>().ok(),
        persistent: true,
    })
}

fn subnet_from_destination_and_mask(destination: &str, mask: &str) -> Option<Ipv4Subnet> {
    let destination = destination.parse::<Ipv4Addr>().ok()?;
    let mask = mask.parse::<Ipv4Addr>().ok()?;
    let prefix = prefix_from_mask(mask)?;
    Some(Ipv4Subnet {
        network: destination,
        prefix,
    })
}

fn parse_gateway(value: &str) -> Option<Ipv4Addr> {
    if value.eq_ignore_ascii_case("on-link") || value.eq_ignore_ascii_case("onlink") {
        None
    } else {
        value.parse::<Ipv4Addr>().ok()
    }
}

fn prefix_from_mask(mask: Ipv4Addr) -> Option<u8> {
    let value = u32::from(mask);
    let mut prefix = 0u8;
    let mut seen_zero = false;
    for bit in (0..32).rev() {
        let set = (value & (1u32 << bit)) != 0;
        if set {
            if seen_zero {
                return None;
            }
            prefix += 1;
        } else {
            seen_zero = true;
        }
    }
    Some(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_route_print_ipv4_active_and_persistent_routes() {
        let routes = parse_windows_ipv4_routes(
            r#"
===========================================================================
IPv4 Route Table
===========================================================================
Active Routes:
Network Destination        Netmask          Gateway       Interface  Metric
          0.0.0.0          0.0.0.0     192.168.1.1   192.168.1.100     25
       10.77.12.0    255.255.255.0         On-link       10.77.12.2      5
       10.77.12.2  255.255.255.255         On-link       10.77.12.2    261
===========================================================================
Persistent Routes:
  Network Address          Netmask  Gateway Address  Metric
       10.77.12.0    255.255.255.0         0.0.0.0       1
===========================================================================
"#,
        );

        assert_eq!(routes.len(), 4);
        assert_eq!(routes[1].destination.to_string(), "10.77.12.0/24");
        assert_eq!(routes[1].interface_ip, Some("10.77.12.2".parse().unwrap()));
        assert_eq!(routes[1].metric, Some(5));
        assert!(!routes[1].persistent);
        assert!(routes[3].persistent);
    }
}
