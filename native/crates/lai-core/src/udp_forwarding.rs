use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UdpForwardObservation {
    pub source: SocketAddr,
    pub destination: SocketAddr,
    pub bytes: usize,
    pub broadcast: bool,
    pub direction: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UdpForwardSummary {
    pub forwarded_packets: u64,
    pub forwarded_bytes: u64,
    pub last_observation: Option<UdpForwardObservation>,
    pub packet_observation_line: Option<String>,
}

pub fn udp_forward_summary(observations: &[UdpForwardObservation]) -> UdpForwardSummary {
    let forwarded_packets = observations.len() as u64;
    let forwarded_bytes = observations
        .iter()
        .map(|observation| observation.bytes as u64)
        .sum();
    let last_observation = observations.last().cloned();
    let packet_observation_line = last_observation
        .as_ref()
        .map(packet_observation_line_from_udp_forward);

    UdpForwardSummary {
        forwarded_packets,
        forwarded_bytes,
        last_observation,
        packet_observation_line,
    }
}

pub fn packet_observation_line_from_udp_forward(observation: &UdpForwardObservation) -> String {
    packet_observation_line_from_transport("udp", observation)
}

pub fn packet_observation_line_from_transport(
    protocol: &str,
    observation: &UdpForwardObservation,
) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}:{}",
        protocol,
        observation.source.ip(),
        observation.destination.ip(),
        observation.destination.port(),
        if observation.broadcast {
            "broadcast"
        } else {
            "unicast"
        },
        observation.direction,
        observation.bytes
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn udp_forward_summary_renders_packet_observation_line() {
        let observation = UdpForwardObservation {
            source: "10.77.12.2:50100".parse().unwrap(),
            destination: "10.77.12.255:39078".parse().unwrap(),
            bytes: 8,
            broadcast: true,
            direction: "outbound".to_owned(),
        };

        let summary = udp_forward_summary(&[observation]);

        assert_eq!(summary.forwarded_packets, 1);
        assert_eq!(summary.forwarded_bytes, 8);
        assert_eq!(
            summary.packet_observation_line.as_deref(),
            Some("udp:10.77.12.2:10.77.12.255:39078:broadcast:outbound:8")
        );
    }

    #[test]
    fn transport_renderer_supports_tcp() {
        let observation = UdpForwardObservation {
            source: "10.77.12.2:50100".parse().unwrap(),
            destination: "10.77.12.3:27015".parse().unwrap(),
            bytes: 8,
            broadcast: false,
            direction: "virtual-adapter".to_owned(),
        };

        assert_eq!(
            packet_observation_line_from_transport("tcp", &observation),
            "tcp:10.77.12.2:10.77.12.3:27015:unicast:virtual-adapter:8"
        );
    }
}
