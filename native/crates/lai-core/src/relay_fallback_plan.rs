use crate::NatTraversalOffer;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RelayFallbackPlan {
    pub status: String,
    pub local_peer_id: String,
    pub remote_peer_id: String,
    pub room_id: String,
    pub p2p_status: String,
    pub p2p_candidate_count: usize,
    pub relay_candidate_count: usize,
    pub selected_relay_endpoints: Vec<String>,
    pub recommended_actions: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn create_relay_fallback_plan(
    local_offer: &NatTraversalOffer,
    remote_offer: &NatTraversalOffer,
    p2p_status: impl Into<String>,
) -> RelayFallbackPlan {
    let p2p_status = normalize_status(p2p_status.into());
    let p2p_candidates = candidate_endpoints(remote_offer, |candidate_type| {
        !candidate_type.eq_ignore_ascii_case("relay")
    });
    let relay_candidates = candidate_endpoints(remote_offer, |candidate_type| {
        candidate_type.eq_ignore_ascii_case("relay")
    });

    let mut warnings = Vec::new();
    if local_offer
        .candidates
        .iter()
        .all(|candidate| !candidate.transport.eq_ignore_ascii_case("udp"))
    {
        warnings.push(
            "Local peer has no UDP candidates; refresh NAT diagnostics before retrying.".to_owned(),
        );
    }
    if remote_offer
        .candidates
        .iter()
        .any(|candidate| !candidate.transport.eq_ignore_ascii_case("udp"))
    {
        warnings.push("Ignored non-UDP remote candidates for game tunnel planning.".to_owned());
    }

    let status = if local_offer.room_id != remote_offer.room_id {
        "config-error"
    } else if local_offer.peer_id == remote_offer.peer_id {
        "config-error"
    } else if is_p2p_success(&p2p_status) {
        "p2p-ready"
    } else if !is_p2p_failure(&p2p_status) && !p2p_candidates.is_empty() {
        "p2p-ready"
    } else if !relay_candidates.is_empty() {
        "relay-available"
    } else if p2p_candidates.is_empty() {
        "no-path"
    } else {
        "needs-relay"
    }
    .to_owned();

    let mut recommended_actions = match status.as_str() {
        "p2p-ready" => vec![
            "Try UDP hole punching and the encrypted P2P handshake first.".to_owned(),
            "If the handshake times out, rerun this plan with p2p-status=failed.".to_owned(),
        ],
        "relay-available" => vec![
            "Use one selected relay endpoint as the next connection path.".to_owned(),
            "Keep Direct IP and host port forwarding as lower-cost alternatives.".to_owned(),
        ],
        "needs-relay" => vec![
            "Enable or configure a relay endpoint for this room.".to_owned(),
            "Ask the host to try UDP port forwarding for the game tunnel port.".to_owned(),
            "Try switching away from restrictive CGNAT, campus, or company networks.".to_owned(),
        ],
        "no-path" => vec![
            "Ask the remote peer to publish fresh UDP candidates through coordination.".to_owned(),
            "Run NAT diagnostics again before attempting a game tunnel.".to_owned(),
        ],
        _ => {
            vec!["Reject this candidate exchange and request matching room/peer offers.".to_owned()]
        }
    };

    if local_offer.room_id != remote_offer.room_id {
        recommended_actions.insert(
            0,
            "Both peers must use NAT offers from the same room id.".to_owned(),
        );
    }
    if local_offer.peer_id == remote_offer.peer_id {
        recommended_actions.insert(
            0,
            "Local and remote offers must belong to different peers.".to_owned(),
        );
    }

    RelayFallbackPlan {
        status,
        local_peer_id: local_offer.peer_id.clone(),
        remote_peer_id: remote_offer.peer_id.clone(),
        room_id: local_offer.room_id.clone(),
        p2p_status,
        p2p_candidate_count: p2p_candidates.len(),
        relay_candidate_count: relay_candidates.len(),
        selected_relay_endpoints: relay_candidates,
        recommended_actions,
        warnings,
    }
}

fn candidate_endpoints(
    offer: &NatTraversalOffer,
    include_candidate_type: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut candidates = offer
        .candidates
        .iter()
        .filter(|candidate| candidate.transport.eq_ignore_ascii_case("udp"))
        .filter(|candidate| include_candidate_type(&candidate.candidate_type))
        .filter(|candidate| seen.insert(candidate.endpoint.clone()))
        .map(|candidate| (candidate.priority, candidate.endpoint.clone()))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    candidates
        .into_iter()
        .map(|(_, endpoint)| endpoint)
        .collect()
}

fn normalize_status(status: String) -> String {
    let trimmed = status.trim();
    if trimmed.is_empty() {
        "unknown".to_owned()
    } else {
        trimmed.to_ascii_lowercase()
    }
}

fn is_p2p_success(status: &str) -> bool {
    matches!(
        status,
        "connected" | "ok" | "ready" | "success" | "succeeded"
    )
}

fn is_p2p_failure(status: &str) -> bool {
    matches!(
        status,
        "failed"
            | "timeout"
            | "timed-out"
            | "blocked"
            | "unreachable"
            | "disconnected"
            | "no-response"
    )
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
            nonce: format!("nonce-{peer_id}"),
            created_at_ms: 1,
            candidates,
        }
    }

    fn candidate(candidate_type: &str, endpoint: &str, priority: u32) -> NatCandidate {
        NatCandidate {
            candidate_type: candidate_type.to_owned(),
            transport: "udp".to_owned(),
            endpoint: endpoint.to_owned(),
            priority,
            source: "test".to_owned(),
        }
    }

    #[test]
    fn fallback_plan_prefers_p2p_before_failure() {
        let local = offer("peer_a", vec![candidate("host", "127.0.0.1:39090", 100)]);
        let remote = offer("peer_b", vec![candidate("host", "127.0.0.1:39091", 100)]);

        let plan = create_relay_fallback_plan(&local, &remote, "unknown");

        assert_eq!(plan.status, "p2p-ready");
        assert_eq!(plan.p2p_candidate_count, 1);
        assert_eq!(plan.relay_candidate_count, 0);
        assert!(plan.selected_relay_endpoints.is_empty());
    }

    #[test]
    fn fallback_plan_uses_relay_after_p2p_failure() {
        let local = offer("peer_a", vec![candidate("host", "127.0.0.1:39090", 100)]);
        let remote = offer(
            "peer_b",
            vec![
                candidate("host", "127.0.0.1:39091", 100),
                candidate("relay", "203.0.113.10:39090", 10),
            ],
        );

        let plan = create_relay_fallback_plan(&local, &remote, "failed");

        assert_eq!(plan.status, "relay-available");
        assert_eq!(plan.p2p_candidate_count, 1);
        assert_eq!(plan.relay_candidate_count, 1);
        assert_eq!(plan.selected_relay_endpoints, vec!["203.0.113.10:39090"]);
    }

    #[test]
    fn fallback_plan_requests_relay_when_direct_path_failed() {
        let local = offer("peer_a", vec![candidate("host", "127.0.0.1:39090", 100)]);
        let remote = offer(
            "peer_b",
            vec![candidate("srflx", "198.51.100.20:44000", 90)],
        );

        let plan = create_relay_fallback_plan(&local, &remote, "timeout");

        assert_eq!(plan.status, "needs-relay");
        assert!(plan
            .recommended_actions
            .iter()
            .any(|action| action.contains("relay endpoint")));
    }
}
