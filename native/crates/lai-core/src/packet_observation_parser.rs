use crate::network_observation::PacketObservation;
use std::net::Ipv4Addr;

pub fn parse_packet_observation_lines(text: &str) -> crate::Result<Vec<PacketObservation>> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(parse_packet_observation_line)
        .collect()
}

pub fn parse_packet_observation_line(value: &str) -> crate::Result<PacketObservation> {
    let parts = value.trim().split(':').collect::<Vec<_>>();
    if parts.len() != 7 {
        return Err(crate::CoreError::Serialization(format!(
            "invalid packet observation `{value}`, expected protocol:source_ip:destination_ip:port:broadcast|unicast:direction:bytes"
        )));
    }
    let broadcast = match parts[4].trim() {
        "broadcast" => true,
        "unicast" => false,
        other => {
            return Err(crate::CoreError::Serialization(format!(
                "unsupported packet observation type `{other}`"
            )))
        }
    };
    Ok(PacketObservation {
        protocol: parts[0].trim().to_owned(),
        source_ip: parse_ipv4(parts[1], "source")?,
        destination_ip: parse_ipv4(parts[2], "destination")?,
        destination_port: parts[3].trim().parse::<u16>().map_err(|err| {
            crate::CoreError::Serialization(format!("invalid packet port `{}`: {err}", parts[3]))
        })?,
        bytes: parts[6].trim().parse::<u32>().map_err(|err| {
            crate::CoreError::Serialization(format!(
                "invalid packet byte count `{}`: {err}",
                parts[6]
            ))
        })?,
        direction: parts[5].trim().to_owned(),
        broadcast,
    })
}

fn parse_ipv4(value: &str, label: &str) -> crate::Result<Ipv4Addr> {
    value.trim().parse::<Ipv4Addr>().map_err(|err| {
        crate::CoreError::Serialization(format!(
            "invalid packet {label} IPv4 address `{}`: {err}",
            value.trim()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_packet_observation_lines() {
        let packets = parse_packet_observation_lines(
            "\
udp:10.77.12.2:10.77.12.255:27015:broadcast:outbound:8
tcp:10.77.12.2:10.77.12.3:27015:unicast:inbound:12
",
        )
        .unwrap();

        assert_eq!(packets.len(), 2);
        assert!(packets[0].broadcast);
        assert_eq!(packets[1].protocol, "tcp");
    }
}
