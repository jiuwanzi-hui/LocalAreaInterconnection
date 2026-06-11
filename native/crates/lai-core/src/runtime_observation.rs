use crate::network_observation::{
    AdapterObservation, NetworkObservationSnapshot, PacketObservation, TunnelObservation,
};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TunnelServiceSnapshot {
    pub service_running: bool,
    pub connected_peer_count: u16,
    pub connection_path: Option<String>,
    pub average_latency_ms: Option<u32>,
    pub packet_loss_percent: Option<f32>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PacketCaptureSummary {
    pub protocol: String,
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
    pub destination_port: u16,
    pub direction: String,
    pub broadcast: bool,
    pub packet_count: u32,
    pub bytes: u32,
}

pub fn tunnel_observation_from_service(snapshot: &TunnelServiceSnapshot) -> TunnelObservation {
    TunnelObservation {
        state: if snapshot.service_running
            && snapshot.connected_peer_count > 0
            && snapshot.last_error.is_none()
        {
            "connected"
        } else if snapshot.service_running {
            "degraded"
        } else {
            "stopped"
        }
        .to_owned(),
        connected_peer_count: snapshot.connected_peer_count,
        latency_ms: snapshot.average_latency_ms,
        packet_loss_percent: snapshot.packet_loss_percent,
        path: snapshot.connection_path.clone(),
    }
}

pub fn packet_observation_from_capture_summary(
    summary: &PacketCaptureSummary,
) -> PacketObservation {
    PacketObservation {
        protocol: summary.protocol.clone(),
        source_ip: summary.source_ip,
        destination_ip: summary.destination_ip,
        destination_port: summary.destination_port,
        bytes: summary.bytes,
        direction: summary.direction.clone(),
        broadcast: summary.broadcast,
    }
}

pub fn network_snapshot_from_runtime(
    adapter: Option<AdapterObservation>,
    tunnel: Option<TunnelServiceSnapshot>,
    captures: &[PacketCaptureSummary],
    expected_peer_count: u16,
    expected_broadcast_ports: Vec<u16>,
    expected_game_ports: Vec<u16>,
) -> NetworkObservationSnapshot {
    NetworkObservationSnapshot {
        adapter,
        tunnel: tunnel.as_ref().map(tunnel_observation_from_service),
        packets: captures
            .iter()
            .map(packet_observation_from_capture_summary)
            .collect(),
        expected_peer_count,
        expected_broadcast_ports,
        expected_game_ports,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network_observation::evaluate_network_observations;

    #[test]
    fn tunnel_service_snapshot_maps_to_tunnel_observation() {
        let snapshot = TunnelServiceSnapshot {
            service_running: true,
            connected_peer_count: 2,
            connection_path: Some("p2p".to_owned()),
            average_latency_ms: Some(18),
            packet_loss_percent: Some(0.0),
            bytes_sent: 128,
            bytes_received: 256,
            last_error: None,
        };

        let observation = tunnel_observation_from_service(&snapshot);

        assert_eq!(observation.state, "connected");
        assert_eq!(observation.connected_peer_count, 2);
        assert_eq!(observation.latency_ms, Some(18));
    }

    #[test]
    fn runtime_snapshot_feeds_network_observation() {
        let tunnel = TunnelServiceSnapshot {
            service_running: true,
            connected_peer_count: 1,
            connection_path: Some("p2p".to_owned()),
            average_latency_ms: Some(12),
            packet_loss_percent: Some(0.0),
            bytes_sent: 128,
            bytes_received: 256,
            last_error: None,
        };
        let captures = vec![
            PacketCaptureSummary {
                protocol: "udp".to_owned(),
                source_ip: "10.77.12.2".parse().unwrap(),
                destination_ip: "10.77.12.255".parse().unwrap(),
                destination_port: 39078,
                direction: "outbound".to_owned(),
                broadcast: true,
                packet_count: 1,
                bytes: 8,
            },
            PacketCaptureSummary {
                protocol: "udp".to_owned(),
                source_ip: "10.77.12.2".parse().unwrap(),
                destination_ip: "10.77.12.1".parse().unwrap(),
                destination_port: 39077,
                direction: "outbound".to_owned(),
                broadcast: false,
                packet_count: 1,
                bytes: 8,
            },
        ];

        let report = evaluate_network_observations(network_snapshot_from_runtime(
            None,
            Some(tunnel),
            &captures,
            1,
            vec![39078],
            vec![39077],
        ));

        assert_eq!(report.diagnostic_snapshot.tunnel.as_deref(), Some("ok"));
        assert_eq!(report.diagnostic_snapshot.p2p.as_deref(), Some("ok"));
        assert_eq!(
            report.diagnostic_snapshot.broadcast.as_deref(),
            Some("seen")
        );
        assert_eq!(
            report.diagnostic_snapshot.game_traffic.as_deref(),
            Some("seen")
        );
    }
}
