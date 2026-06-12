use crate::{CoordinationStore, Ipv4Subnet};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoordinationRoomMemberView {
    pub peer_id: String,
    pub virtual_ip: Option<Ipv4Addr>,
    pub status: String,
    pub candidate_count: usize,
    pub preferred_endpoint: Option<String>,
    pub last_seen_ms: u128,
    pub expires_at_ms: u128,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoordinationRoomView {
    pub status: String,
    pub room_id: String,
    pub local_peer_id: String,
    pub member_count: usize,
    pub online_count: usize,
    pub expired_count: usize,
    pub members: Vec<CoordinationRoomMemberView>,
    pub next_action: String,
}

pub fn coordination_room_view(
    store: &CoordinationStore,
    room_id: impl Into<String>,
    local_peer_id: impl Into<String>,
    virtual_subnet: Ipv4Subnet,
    now_ms: u128,
) -> CoordinationRoomView {
    let room_id = room_id.into();
    let local_peer_id = local_peer_id.into();
    let mut members = store
        .rooms
        .iter()
        .find(|room| room.room_id == room_id)
        .map(|room| {
            room.peers
                .iter()
                .map(|peer| {
                    let online = peer.expires_at_ms > now_ms;
                    let candidate_count = peer
                        .offer
                        .as_ref()
                        .map(|offer| offer.candidates.len())
                        .unwrap_or(0);
                    let preferred_endpoint = peer.offer.as_ref().and_then(|offer| {
                        offer
                            .candidates
                            .iter()
                            .max_by_key(|candidate| candidate.priority)
                            .map(|candidate| candidate.endpoint.clone())
                    });
                    CoordinationRoomMemberView {
                        peer_id: peer.peer_id.clone(),
                        virtual_ip: peer_virtual_ip(virtual_subnet, &peer.peer_id),
                        status: if online { "online" } else { "expired" }.to_owned(),
                        candidate_count,
                        preferred_endpoint,
                        last_seen_ms: peer.last_seen_ms,
                        expires_at_ms: peer.expires_at_ms,
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    members.sort_by(|left, right| left.peer_id.cmp(&right.peer_id));

    let online_count = members
        .iter()
        .filter(|member| member.status == "online")
        .count();
    let expired_count = members
        .iter()
        .filter(|member| member.status == "expired")
        .count();
    let has_local = members.iter().any(|member| member.peer_id == local_peer_id);
    let remote_online = members
        .iter()
        .any(|member| member.peer_id != local_peer_id && member.status == "online");
    let missing_candidates = members
        .iter()
        .any(|member| member.status == "online" && member.candidate_count == 0);

    let next_action = if members.is_empty() {
        "Publish a local offer to the coordination server.".to_owned()
    } else if !has_local {
        "Publish or heartbeat the local peer before waiting for others.".to_owned()
    } else if !remote_online {
        "Wait for remote peers to publish offers, then start runtime bootstrap.".to_owned()
    } else if missing_candidates {
        "Refresh NAT candidates for peers that have no endpoints.".to_owned()
    } else {
        "Start or refresh runtime bootstrap with the listed peers.".to_owned()
    };

    CoordinationRoomView {
        status: if remote_online { "ready" } else { "waiting" }.to_owned(),
        room_id,
        local_peer_id,
        member_count: members.len(),
        online_count,
        expired_count,
        members,
        next_action,
    }
}

fn peer_virtual_ip(subnet: Ipv4Subnet, peer_id: &str) -> Option<Ipv4Addr> {
    if peer_id.is_empty() {
        return None;
    }
    let hash = peer_id.bytes().fold(0u32, |value, byte| {
        value.wrapping_mul(31).wrapping_add(byte as u32)
    });
    let offset = 2 + (hash % 200);
    let octets = subnet.network.octets();
    Some(Ipv4Addr::new(octets[0], octets[1], octets[2], offset as u8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        create_coordination_store, create_nat_traversal_offer, publish_coordination_offer,
    };

    #[test]
    fn room_view_reports_online_members_and_next_action() {
        let mut store = create_coordination_store();
        let offer_a = create_nat_traversal_offer(
            "room",
            "peer_a",
            "a",
            10,
            "127.0.0.1:10000".parse().unwrap(),
            None,
            vec![],
        );
        let offer_b = create_nat_traversal_offer(
            "room",
            "peer_b",
            "b",
            20,
            "127.0.0.1:10001".parse().unwrap(),
            None,
            vec![],
        );
        publish_coordination_offer(&mut store, offer_a, 100, 1000);
        publish_coordination_offer(&mut store, offer_b, 110, 1000);

        let view = coordination_room_view(
            &store,
            "room",
            "peer_a",
            "10.77.12.0/24".parse().unwrap(),
            120,
        );

        assert_eq!(view.status, "ready");
        assert_eq!(view.member_count, 2);
        assert_eq!(view.online_count, 2);
        assert_eq!(view.expired_count, 0);
        assert_eq!(view.members[0].peer_id, "peer_a");
        assert_eq!(
            view.members[1].preferred_endpoint.as_deref(),
            Some("127.0.0.1:10001")
        );
        assert!(view.next_action.contains("runtime bootstrap"));
    }

    #[test]
    fn room_view_reports_expired_members() {
        let mut store = create_coordination_store();
        let offer = create_nat_traversal_offer(
            "room",
            "peer_a",
            "a",
            10,
            "127.0.0.1:10000".parse().unwrap(),
            None,
            vec![],
        );
        publish_coordination_offer(&mut store, offer, 100, 10);

        let view = coordination_room_view(
            &store,
            "room",
            "peer_a",
            "10.77.12.0/24".parse().unwrap(),
            200,
        );

        assert_eq!(view.status, "waiting");
        assert_eq!(view.online_count, 0);
        assert_eq!(view.expired_count, 1);
        assert_eq!(view.members[0].status, "expired");
    }
}
