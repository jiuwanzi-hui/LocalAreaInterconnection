use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NatCandidate {
    pub candidate_type: String,
    pub transport: String,
    pub endpoint: String,
    pub priority: u32,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NatTraversalOffer {
    pub schema_version: u16,
    pub room_id: String,
    pub peer_id: String,
    pub nonce: String,
    pub created_at_ms: u128,
    pub candidates: Vec<NatCandidate>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NatPunchPlan {
    pub status: String,
    pub local_peer_id: String,
    pub remote_peer_id: String,
    pub target_endpoints: Vec<String>,
    pub attempt_count: u16,
    pub interval_ms: u64,
    pub next_action: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoordinationMessage {
    pub schema_version: u16,
    pub message_type: String,
    pub room_id: String,
    pub peer_id: String,
    pub sequence: u64,
    pub sent_at_ms: u128,
    pub offer: Option<NatTraversalOffer>,
}

pub fn create_nat_traversal_offer(
    room_id: impl Into<String>,
    peer_id: impl Into<String>,
    nonce: impl Into<String>,
    created_at_ms: u128,
    local_endpoint: SocketAddr,
    observed_endpoint: Option<SocketAddr>,
    relay_endpoints: Vec<SocketAddr>,
) -> NatTraversalOffer {
    let mut candidates = Vec::new();
    candidates.push(candidate("host", local_endpoint, 100, "local-socket"));
    if let Some(endpoint) = observed_endpoint {
        candidates.push(candidate("srflx", endpoint, 90, "observed-endpoint"));
    }
    for endpoint in relay_endpoints {
        candidates.push(candidate("relay", endpoint, 10, "relay"));
    }
    deduplicate_candidates(&mut candidates);
    candidates.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.endpoint.cmp(&right.endpoint))
    });

    NatTraversalOffer {
        schema_version: 1,
        room_id: room_id.into(),
        peer_id: peer_id.into(),
        nonce: nonce.into(),
        created_at_ms,
        candidates,
    }
}

pub fn create_coordination_message(
    message_type: impl Into<String>,
    room_id: impl Into<String>,
    peer_id: impl Into<String>,
    sequence: u64,
    sent_at_ms: u128,
    offer: Option<NatTraversalOffer>,
) -> CoordinationMessage {
    CoordinationMessage {
        schema_version: 1,
        message_type: message_type.into(),
        room_id: room_id.into(),
        peer_id: peer_id.into(),
        sequence,
        sent_at_ms,
        offer,
    }
}

pub fn create_nat_punch_plan(
    local_offer: &NatTraversalOffer,
    remote_offer: &NatTraversalOffer,
    attempt_count: u16,
    interval_ms: u64,
) -> NatPunchPlan {
    let mut targets = remote_offer
        .candidates
        .iter()
        .filter(|candidate| candidate.transport.eq_ignore_ascii_case("udp"))
        .map(|candidate| candidate.endpoint.clone())
        .collect::<Vec<_>>();
    targets.sort();
    targets.dedup();

    let status = if local_offer.room_id != remote_offer.room_id {
        "room-mismatch"
    } else if local_offer.peer_id == remote_offer.peer_id {
        "same-peer"
    } else if targets.is_empty() {
        "no-candidates"
    } else {
        "ready"
    }
    .to_owned();

    let next_action = match status.as_str() {
        "ready" => "Send small UDP punch packets to every target endpoint at the configured interval, then start the encrypted P2P handshake.".to_owned(),
        "room-mismatch" => "Reject this candidate exchange; both peers must use the same room id.".to_owned(),
        "same-peer" => "Reject this candidate exchange; a peer cannot punch to itself.".to_owned(),
        _ => "Wait for the remote peer to publish at least one UDP candidate endpoint.".to_owned(),
    };

    NatPunchPlan {
        status,
        local_peer_id: local_offer.peer_id.clone(),
        remote_peer_id: remote_offer.peer_id.clone(),
        target_endpoints: targets,
        attempt_count: attempt_count.max(1),
        interval_ms,
        next_action,
    }
}

fn candidate(
    candidate_type: impl Into<String>,
    endpoint: SocketAddr,
    priority: u32,
    source: impl Into<String>,
) -> NatCandidate {
    NatCandidate {
        candidate_type: candidate_type.into(),
        transport: "udp".to_owned(),
        endpoint: endpoint.to_string(),
        priority,
        source: source.into(),
    }
}

fn deduplicate_candidates(candidates: &mut Vec<NatCandidate>) {
    let mut seen = HashSet::new();
    candidates.retain(|candidate| {
        seen.insert((
            candidate.transport.to_ascii_lowercase(),
            candidate.endpoint.clone(),
            candidate.candidate_type.clone(),
        ))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offer_deduplicates_and_sorts_candidates() {
        let offer = create_nat_traversal_offer(
            "room",
            "peer_a",
            "nonce",
            123,
            "127.0.0.1:10000".parse().unwrap(),
            Some("127.0.0.1:10000".parse().unwrap()),
            vec!["10.0.0.1:20000".parse().unwrap()],
        );

        assert_eq!(offer.schema_version, 1);
        assert_eq!(offer.candidates.len(), 3);
        assert_eq!(offer.candidates[0].candidate_type, "host");
        assert_eq!(offer.candidates[2].candidate_type, "relay");
    }

    #[test]
    fn punch_plan_targets_remote_candidates() {
        let local = create_nat_traversal_offer(
            "room",
            "peer_a",
            "a",
            1,
            "127.0.0.1:10000".parse().unwrap(),
            None,
            vec![],
        );
        let remote = create_nat_traversal_offer(
            "room",
            "peer_b",
            "b",
            1,
            "127.0.0.1:10001".parse().unwrap(),
            Some("192.0.2.10:40000".parse().unwrap()),
            vec![],
        );

        let plan = create_nat_punch_plan(&local, &remote, 3, 25);

        assert_eq!(plan.status, "ready");
        assert_eq!(plan.target_endpoints.len(), 2);
        assert_eq!(plan.attempt_count, 3);
    }

    #[test]
    fn coordination_message_wraps_offer() {
        let offer = create_nat_traversal_offer(
            "room",
            "peer_a",
            "a",
            1,
            "127.0.0.1:10000".parse().unwrap(),
            None,
            vec![],
        );

        let message =
            create_coordination_message("candidate-offer", "room", "peer_a", 7, 9, Some(offer));

        assert_eq!(message.message_type, "candidate-offer");
        assert_eq!(message.sequence, 7);
        assert!(message.offer.is_some());
    }
}
