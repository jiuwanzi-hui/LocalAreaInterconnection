use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WindowsNetstatEndpoint {
    pub protocol: String,
    pub local_address: Option<IpAddr>,
    pub local_port: Option<u16>,
    pub foreign_address: Option<IpAddr>,
    pub foreign_port: Option<u16>,
    pub state: Option<String>,
    pub pid: Option<u32>,
}

pub fn parse_windows_netstat_ano(output: &str) -> Vec<WindowsNetstatEndpoint> {
    output
        .lines()
        .filter_map(parse_netstat_line)
        .collect::<Vec<_>>()
}

fn parse_netstat_line(line: &str) -> Option<WindowsNetstatEndpoint> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    let protocol = parts.first()?.to_ascii_lowercase();
    if protocol != "tcp" && protocol != "udp" {
        return None;
    }

    match protocol.as_str() {
        "tcp" if parts.len() >= 5 => {
            let local = parse_endpoint(parts[1]);
            let foreign = parse_endpoint(parts[2]);
            Some(WindowsNetstatEndpoint {
                protocol,
                local_address: local.0,
                local_port: local.1,
                foreign_address: foreign.0,
                foreign_port: foreign.1,
                state: Some(parts[3].to_ascii_lowercase()),
                pid: parts[4].parse().ok(),
            })
        }
        "udp" if parts.len() >= 4 => {
            let local = parse_endpoint(parts[1]);
            let (foreign, pid_part) = if parts.len() >= 5 {
                (parse_endpoint(parts[2]), parts[4])
            } else {
                ((None, None), parts[3])
            };
            Some(WindowsNetstatEndpoint {
                protocol,
                local_address: local.0,
                local_port: local.1,
                foreign_address: foreign.0,
                foreign_port: foreign.1,
                state: None,
                pid: pid_part.parse().ok(),
            })
        }
        _ => None,
    }
}

fn parse_endpoint(value: &str) -> (Option<IpAddr>, Option<u16>) {
    if matches!(value, "*:*" | "*") {
        return (None, None);
    }

    if let Some(end) = value.find("]:") {
        let address = &value[1..end];
        let port = &value[end + 2..];
        return (parse_address(address), parse_port(port));
    }

    if let Some((address, port)) = value.rsplit_once(':') {
        return (parse_address(address), parse_port(port));
    }

    (parse_address(value), None)
}

fn parse_address(value: &str) -> Option<IpAddr> {
    if matches!(value, "*" | "0" | "") {
        return None;
    }
    value.parse().ok()
}

fn parse_port(value: &str) -> Option<u16> {
    if matches!(value, "*" | "") {
        return None;
    }
    value.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tcp_and_udp_netstat_rows() {
        let endpoints = parse_windows_netstat_ano(
            r#"
Active Connections

  Proto  Local Address          Foreign Address        State           PID
  TCP    0.0.0.0:27015          0.0.0.0:0              LISTENING       4242
  UDP    0.0.0.0:27016          *:*                                    4243
"#,
        );

        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].protocol, "tcp");
        assert_eq!(endpoints[0].local_port, Some(27015));
        assert_eq!(endpoints[0].state.as_deref(), Some("listening"));
        assert_eq!(endpoints[0].pid, Some(4242));
        assert_eq!(endpoints[1].protocol, "udp");
        assert_eq!(endpoints[1].local_port, Some(27016));
        assert_eq!(endpoints[1].pid, Some(4243));
    }

    #[test]
    fn parses_ipv6_bracket_endpoints() {
        let endpoints = parse_windows_netstat_ano(
            r#"
  Proto  Local Address          Foreign Address        State           PID
  TCP    [::]:7777              [::]:0                 LISTENING       4
"#,
        );

        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].local_address.unwrap().to_string(), "::");
        assert_eq!(endpoints[0].local_port, Some(7777));
    }
}
