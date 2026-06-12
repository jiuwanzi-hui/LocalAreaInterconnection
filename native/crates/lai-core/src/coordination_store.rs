use crate::NatTraversalOffer;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoordinationStore {
    pub schema_version: u16,
    pub rooms: Vec<CoordinationRoom>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoordinationRoom {
    pub room_id: String,
    pub created_at_ms: u128,
    pub updated_at_ms: u128,
    pub peers: Vec<CoordinationPeer>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoordinationPeer {
    pub peer_id: String,
    pub last_seen_ms: u128,
    pub expires_at_ms: u128,
    pub offer: Option<NatTraversalOffer>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoordinationStoreUpdate {
    pub status: String,
    pub room_id: String,
    pub peer_id: String,
    pub expires_at_ms: u128,
    pub remote_offer_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoordinationFetchResult {
    pub status: String,
    pub room_id: String,
    pub peer_id: String,
    pub offers: Vec<NatTraversalOffer>,
    pub expired_peer_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoordinationPruneReport {
    pub status: String,
    pub expired_peer_count: usize,
    pub removed_room_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoordinationLeaveReport {
    pub status: String,
    pub room_id: String,
    pub peer_id: String,
    pub peer_removed: bool,
    pub room_removed: bool,
    pub remaining_peer_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CoordinationCloseReport {
    pub status: String,
    pub room_id: String,
    pub room_removed: bool,
    pub removed_peer_count: usize,
}

pub fn create_coordination_store() -> CoordinationStore {
    CoordinationStore {
        schema_version: 1,
        rooms: Vec::new(),
    }
}

pub fn publish_coordination_offer(
    store: &mut CoordinationStore,
    offer: NatTraversalOffer,
    now_ms: u128,
    ttl_ms: u128,
) -> CoordinationStoreUpdate {
    let room_id = offer.room_id.clone();
    let peer_id = offer.peer_id.clone();
    let expires_at_ms = now_ms.saturating_add(ttl_ms.max(1));
    let room = get_or_insert_room(store, &room_id, now_ms);
    room.updated_at_ms = now_ms;
    let peer = get_or_insert_peer(room, &peer_id, now_ms, expires_at_ms);
    peer.last_seen_ms = now_ms;
    peer.expires_at_ms = expires_at_ms;
    peer.offer = Some(offer);
    let remote_offer_count = room
        .peers
        .iter()
        .filter(|peer| peer.peer_id != peer_id)
        .filter(|peer| peer.expires_at_ms > now_ms)
        .filter(|peer| peer.offer.is_some())
        .count();

    CoordinationStoreUpdate {
        status: "ok".to_owned(),
        room_id,
        peer_id,
        expires_at_ms,
        remote_offer_count,
    }
}

pub fn heartbeat_coordination_peer(
    store: &mut CoordinationStore,
    room_id: impl Into<String>,
    peer_id: impl Into<String>,
    now_ms: u128,
    ttl_ms: u128,
) -> CoordinationStoreUpdate {
    let room_id = room_id.into();
    let peer_id = peer_id.into();
    let expires_at_ms = now_ms.saturating_add(ttl_ms.max(1));
    let room = get_or_insert_room(store, &room_id, now_ms);
    room.updated_at_ms = now_ms;
    let peer = get_or_insert_peer(room, &peer_id, now_ms, expires_at_ms);
    peer.last_seen_ms = now_ms;
    peer.expires_at_ms = expires_at_ms;
    let remote_offer_count = room
        .peers
        .iter()
        .filter(|peer| peer.peer_id != peer_id)
        .filter(|peer| peer.expires_at_ms > now_ms)
        .filter(|peer| peer.offer.is_some())
        .count();

    CoordinationStoreUpdate {
        status: "ok".to_owned(),
        room_id,
        peer_id,
        expires_at_ms,
        remote_offer_count,
    }
}

pub fn fetch_coordination_offers(
    store: &mut CoordinationStore,
    room_id: impl Into<String>,
    peer_id: impl Into<String>,
    now_ms: u128,
) -> CoordinationFetchResult {
    let room_id = room_id.into();
    let peer_id = peer_id.into();
    let prune_report = prune_expired_coordination_peers(store, now_ms);
    let offers = store
        .rooms
        .iter()
        .find(|room| room.room_id == room_id)
        .map(|room| {
            room.peers
                .iter()
                .filter(|peer| peer.peer_id != peer_id)
                .filter(|peer| peer.expires_at_ms > now_ms)
                .filter_map(|peer| peer.offer.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    CoordinationFetchResult {
        status: if offers.is_empty() {
            "empty".to_owned()
        } else {
            "ok".to_owned()
        },
        room_id,
        peer_id,
        offers,
        expired_peer_count: prune_report.expired_peer_count,
    }
}

pub fn prune_expired_coordination_peers(
    store: &mut CoordinationStore,
    now_ms: u128,
) -> CoordinationPruneReport {
    let mut expired_peer_count = 0usize;
    for room in &mut store.rooms {
        let before = room.peers.len();
        room.peers.retain(|peer| peer.expires_at_ms > now_ms);
        expired_peer_count += before.saturating_sub(room.peers.len());
    }
    let before_rooms = store.rooms.len();
    store.rooms.retain(|room| !room.peers.is_empty());
    let removed_room_count = before_rooms.saturating_sub(store.rooms.len());

    CoordinationPruneReport {
        status: "ok".to_owned(),
        expired_peer_count,
        removed_room_count,
    }
}

pub fn leave_coordination_room(
    store: &mut CoordinationStore,
    room_id: impl Into<String>,
    peer_id: impl Into<String>,
    now_ms: u128,
) -> CoordinationLeaveReport {
    let room_id = room_id.into();
    let peer_id = peer_id.into();
    let Some(room_index) = store.rooms.iter().position(|room| room.room_id == room_id) else {
        return CoordinationLeaveReport {
            status: "not-found".to_owned(),
            room_id,
            peer_id,
            peer_removed: false,
            room_removed: false,
            remaining_peer_count: 0,
        };
    };

    let room = &mut store.rooms[room_index];
    let before = room.peers.len();
    room.peers.retain(|peer| peer.peer_id != peer_id);
    let peer_removed = room.peers.len() != before;
    room.updated_at_ms = now_ms;
    let remaining_peer_count = room.peers.len();
    let room_removed = room.peers.is_empty();
    if room_removed {
        store.rooms.remove(room_index);
    }

    CoordinationLeaveReport {
        status: if peer_removed { "ok" } else { "not-found" }.to_owned(),
        room_id,
        peer_id,
        peer_removed,
        room_removed,
        remaining_peer_count,
    }
}

pub fn close_coordination_room(
    store: &mut CoordinationStore,
    room_id: impl Into<String>,
) -> CoordinationCloseReport {
    let room_id = room_id.into();
    let Some(room_index) = store.rooms.iter().position(|room| room.room_id == room_id) else {
        return CoordinationCloseReport {
            status: "not-found".to_owned(),
            room_id,
            room_removed: false,
            removed_peer_count: 0,
        };
    };
    let removed_peer_count = store.rooms[room_index].peers.len();
    store.rooms.remove(room_index);

    CoordinationCloseReport {
        status: "ok".to_owned(),
        room_id,
        room_removed: true,
        removed_peer_count,
    }
}

fn get_or_insert_room<'a>(
    store: &'a mut CoordinationStore,
    room_id: &str,
    now_ms: u128,
) -> &'a mut CoordinationRoom {
    if let Some(index) = store.rooms.iter().position(|room| room.room_id == room_id) {
        return &mut store.rooms[index];
    }
    store.rooms.push(CoordinationRoom {
        room_id: room_id.to_owned(),
        created_at_ms: now_ms,
        updated_at_ms: now_ms,
        peers: Vec::new(),
    });
    store.rooms.last_mut().expect("room inserted")
}

fn get_or_insert_peer<'a>(
    room: &'a mut CoordinationRoom,
    peer_id: &str,
    now_ms: u128,
    expires_at_ms: u128,
) -> &'a mut CoordinationPeer {
    if let Some(index) = room.peers.iter().position(|peer| peer.peer_id == peer_id) {
        return &mut room.peers[index];
    }
    room.peers.push(CoordinationPeer {
        peer_id: peer_id.to_owned(),
        last_seen_ms: now_ms,
        expires_at_ms,
        offer: None,
    });
    room.peers.last_mut().expect("peer inserted")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_nat_traversal_offer;

    #[test]
    fn coordination_store_publishes_fetches_and_expires_offers() {
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

        let update_a = publish_coordination_offer(&mut store, offer_a, 100, 50);
        let update_b = publish_coordination_offer(&mut store, offer_b, 110, 50);
        let fetched = fetch_coordination_offers(&mut store, "room", "peer_a", 120);
        let expired = fetch_coordination_offers(&mut store, "room", "peer_a", 170);

        assert_eq!(update_a.remote_offer_count, 0);
        assert_eq!(update_b.remote_offer_count, 1);
        assert_eq!(fetched.status, "ok");
        assert_eq!(fetched.offers.len(), 1);
        assert_eq!(fetched.offers[0].peer_id, "peer_b");
        assert_eq!(expired.status, "empty");
        assert_eq!(expired.expired_peer_count, 2);
    }

    #[test]
    fn coordination_store_leaves_and_closes_rooms() {
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

        let leave = leave_coordination_room(&mut store, "room", "peer_a", 120);
        let fetched = fetch_coordination_offers(&mut store, "room", "peer_b", 130);
        let close = close_coordination_room(&mut store, "room");
        let close_again = close_coordination_room(&mut store, "room");

        assert_eq!(leave.status, "ok");
        assert!(leave.peer_removed);
        assert!(!leave.room_removed);
        assert_eq!(leave.remaining_peer_count, 1);
        assert_eq!(fetched.status, "empty");
        assert_eq!(close.status, "ok");
        assert!(close.room_removed);
        assert_eq!(close.removed_peer_count, 1);
        assert_eq!(close_again.status, "not-found");
        assert_eq!(store.rooms.len(), 0);
    }
}
