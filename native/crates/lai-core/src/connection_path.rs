use crate::{create_relay_fallback_plan, NatTraversalOffer, RelayFallbackPlan};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConnectionPathReport {
    pub status: String,
    pub room_id: String,
    pub local_peer_id: String,
    pub remote_peer_id: String,
    pub p2p_status: String,
    pub selected_path: String,
    pub local_nat_assessment: String,
    pub remote_nat_assessment: String,
    pub local_udp_candidate_count: usize,
    pub remote_udp_candidate_count: usize,
    #[serde(default)]
    pub local_host_candidate_count: usize,
    #[serde(default)]
    pub local_srflx_candidate_count: usize,
    #[serde(default)]
    pub local_relay_candidate_count: usize,
    #[serde(default)]
    pub remote_host_candidate_count: usize,
    #[serde(default)]
    pub remote_srflx_candidate_count: usize,
    pub remote_p2p_candidate_count: usize,
    pub remote_relay_candidate_count: usize,
    pub selected_endpoints: Vec<String>,
    pub relay_fallback: RelayFallbackPlan,
    pub recommended_actions: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn evaluate_connection_path(
    local_offer: &NatTraversalOffer,
    remote_offer: &NatTraversalOffer,
    p2p_status: impl Into<String>,
) -> ConnectionPathReport {
    let p2p_status = normalize_status(p2p_status.into());
    let relay_fallback = create_relay_fallback_plan(local_offer, remote_offer, p2p_status.clone());
    let local_udp_candidate_count = udp_candidate_count(local_offer);
    let remote_udp_candidate_count = udp_candidate_count(remote_offer);
    let local_host_candidate_count = candidate_type_count(local_offer, "host");
    let local_srflx_candidate_count = candidate_type_count(local_offer, "srflx");
    let local_relay_candidate_count = candidate_type_count(local_offer, "relay");
    let remote_host_candidate_count = candidate_type_count(remote_offer, "host");
    let remote_srflx_candidate_count = candidate_type_count(remote_offer, "srflx");
    let remote_p2p_candidates = candidate_endpoints(remote_offer, |candidate_type| {
        !candidate_type.eq_ignore_ascii_case("relay")
    });
    let remote_relay_candidates = relay_fallback.selected_relay_endpoints.clone();
    let selected_path = match relay_fallback.status.as_str() {
        "p2p-ready" => "p2p",
        "relay-available" => "relay",
        _ => "none",
    }
    .to_owned();
    let selected_endpoints = if selected_path == "relay" {
        remote_relay_candidates.clone()
    } else if selected_path == "p2p" {
        remote_p2p_candidates.clone()
    } else {
        Vec::new()
    };
    let status = match relay_fallback.status.as_str() {
        "p2p-ready" => "p2p-candidate-ready",
        "relay-available" => "relay-ready",
        "needs-relay" => "needs-relay",
        "no-path" => "no-path",
        _ => "config-error",
    }
    .to_owned();

    let mut warnings = relay_fallback.warnings.clone();
    let local_nat_assessment = nat_assessment(local_offer);
    let remote_nat_assessment = nat_assessment(remote_offer);
    if local_nat_assessment == "private-or-lan-only"
        || remote_nat_assessment == "private-or-lan-only"
    {
        warnings.push(
            "One peer only exposed private or LAN host candidates; P2P may require coordination, port forwarding, or relay.".to_owned(),
        );
    }
    if selected_path == "relay" {
        warnings.push("Relay path may add latency and bandwidth cost.".to_owned());
    }

    let mut recommended_actions = relay_fallback.recommended_actions.clone();
    recommended_actions.insert(
        0,
        match selected_path.as_str() {
            "p2p" => {
                "Attempt UDP punch and encrypted P2P handshake before starting the game tunnel."
            }
            "relay" => "Use the selected relay endpoint if direct P2P has failed.",
            _ => "Refresh NAT candidates or configure a relay before starting the game tunnel.",
        }
        .to_owned(),
    );

    ConnectionPathReport {
        status,
        room_id: local_offer.room_id.clone(),
        local_peer_id: local_offer.peer_id.clone(),
        remote_peer_id: remote_offer.peer_id.clone(),
        p2p_status,
        selected_path,
        local_nat_assessment,
        remote_nat_assessment,
        local_udp_candidate_count,
        remote_udp_candidate_count,
        local_host_candidate_count,
        local_srflx_candidate_count,
        local_relay_candidate_count,
        remote_host_candidate_count,
        remote_srflx_candidate_count,
        remote_p2p_candidate_count: remote_p2p_candidates.len(),
        remote_relay_candidate_count: remote_relay_candidates.len(),
        selected_endpoints,
        relay_fallback,
        recommended_actions,
        warnings,
    }
}

fn udp_candidate_count(offer: &NatTraversalOffer) -> usize {
    offer
        .candidates
        .iter()
        .filter(|candidate| candidate.transport.eq_ignore_ascii_case("udp"))
        .count()
}

fn candidate_type_count(offer: &NatTraversalOffer, expected_type: &str) -> usize {
    offer
        .candidates
        .iter()
        .filter(|candidate| candidate.transport.eq_ignore_ascii_case("udp"))
        .filter(|candidate| candidate.candidate_type.eq_ignore_ascii_case(expected_type))
        .count()
}

fn candidate_endpoints(
    offer: &NatTraversalOffer,
    include_candidate_type: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut endpoints = offer
        .candidates
        .iter()
        .filter(|candidate| candidate.transport.eq_ignore_ascii_case("udp"))
        .filter(|candidate| include_candidate_type(&candidate.candidate_type))
        .map(|candidate| {
            (
                p2p_candidate_rank(&candidate.candidate_type, &candidate.source),
                candidate.priority,
                candidate.endpoint.clone(),
            )
        })
        .collect::<Vec<_>>();
    endpoints.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    endpoints.dedup_by(|left, right| left.2 == right.2);
    endpoints
        .into_iter()
        .map(|(_, _, endpoint)| endpoint)
        .collect()
}

fn p2p_candidate_rank(candidate_type: &str, source: &str) -> u8 {
    if candidate_type.eq_ignore_ascii_case("srflx") {
        if source.eq_ignore_ascii_case("upnp-port-mapping") {
            4
        } else {
            3
        }
    } else if candidate_type.eq_ignore_ascii_case("host") {
        2
    } else if candidate_type.eq_ignore_ascii_case("relay") {
        1
    } else {
        0
    }
}

fn nat_assessment(offer: &NatTraversalOffer) -> String {
    let has_udp = offer
        .candidates
        .iter()
        .any(|candidate| candidate.transport.eq_ignore_ascii_case("udp"));
    if !has_udp {
        return "no-udp-candidates".to_owned();
    }
    let has_srflx = offer.candidates.iter().any(|candidate| {
        candidate.transport.eq_ignore_ascii_case("udp")
            && candidate.candidate_type.eq_ignore_ascii_case("srflx")
    });
    let has_relay = offer.candidates.iter().any(|candidate| {
        candidate.transport.eq_ignore_ascii_case("udp")
            && candidate.candidate_type.eq_ignore_ascii_case("relay")
    });
    let has_host = offer.candidates.iter().any(|candidate| {
        candidate.transport.eq_ignore_ascii_case("udp")
            && candidate.candidate_type.eq_ignore_ascii_case("host")
    });
    match (has_host, has_srflx, has_relay) {
        (_, true, true) => "nat-mapped-with-relay",
        (_, true, false) => "nat-mapped",
        (false, false, true) => "relay-only",
        (true, false, true) => "private-or-lan-with-relay",
        (true, false, false) => "private-or-lan-only",
        _ => "unknown",
    }
    .to_owned()
}

fn normalize_status(status: String) -> String {
    let trimmed = status.trim();
    if trimmed.is_empty() {
        "unknown".to_owned()
    } else {
        trimmed.to_ascii_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NatCandidate, NatTraversalOffer};

    fn offer(peer_id: &str, candidates: Vec<NatCandidate>) -> NatTraversalOffer {
        NatTraversalOffer {
            schema_version: 1,
            room_id: "room_test".to_owned(),
            peer_id: peer_id.to_owned(),
            virtual_ip: None,
            nonce: format!("nonce-{peer_id}"),
            created_at_ms: 1,
            candidates,
        }
    }

    fn candidate(candidate_type: &str, endpoint: &str, priority: u32) -> NatCandidate {
        candidate_from_source(candidate_type, endpoint, priority, "test")
    }

    fn candidate_from_source(
        candidate_type: &str,
        endpoint: &str,
        priority: u32,
        source: &str,
    ) -> NatCandidate {
        NatCandidate {
            candidate_type: candidate_type.to_owned(),
            transport: "udp".to_owned(),
            endpoint: endpoint.to_owned(),
            priority,
            source: source.to_owned(),
        }
    }

    #[test]
    fn connection_path_prefers_p2p_candidates_before_failure() {
        let local = offer("peer_a", vec![candidate("host", "10.0.0.2:39090", 100)]);
        let remote = offer(
            "peer_b",
            vec![candidate("srflx", "198.51.100.20:44000", 90)],
        );

        let report = evaluate_connection_path(&local, &remote, "unknown");

        assert_eq!(report.status, "p2p-candidate-ready");
        assert_eq!(report.selected_path, "p2p");
        assert_eq!(report.remote_nat_assessment, "nat-mapped");
        assert_eq!(report.selected_endpoints, vec!["198.51.100.20:44000"]);
    }

    #[test]
    fn connection_path_orders_routable_p2p_candidates_before_host() {
        let local = offer("peer_a", vec![candidate("host", "10.0.0.2:39090", 100)]);
        let remote = offer(
            "peer_b",
            vec![
                candidate("host", "192.168.1.20:39090", 100),
                candidate("srflx", "198.51.100.20:44000", 90),
                candidate_from_source("srflx", "198.51.100.20:39090", 90, "upnp-port-mapping"),
                candidate("relay", "203.0.113.10:39091", 10),
            ],
        );

        let report = evaluate_connection_path(&local, &remote, "unknown");

        assert_eq!(report.selected_path, "p2p");
        assert_eq!(
            report.selected_endpoints,
            vec![
                "198.51.100.20:39090",
                "198.51.100.20:44000",
                "192.168.1.20:39090",
            ]
        );
    }

    #[test]
    fn connection_path_uses_relay_after_p2p_failure() {
        let local = offer("peer_a", vec![candidate("host", "10.0.0.2:39090", 100)]);
        let remote = offer(
            "peer_b",
            vec![
                candidate("srflx", "198.51.100.20:44000", 90),
                candidate("relay", "203.0.113.10:39090", 10),
            ],
        );

        let report = evaluate_connection_path(&local, &remote, "failed");

        assert_eq!(report.status, "relay-ready");
        assert_eq!(report.selected_path, "relay");
        assert_eq!(report.remote_relay_candidate_count, 1);
        assert_eq!(report.selected_endpoints, vec!["203.0.113.10:39090"]);
    }
}
