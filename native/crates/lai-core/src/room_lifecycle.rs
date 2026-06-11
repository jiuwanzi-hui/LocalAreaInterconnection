use crate::ip::{peer_address, Ipv4Subnet};
use crate::room::Room;
use crate::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum RoomLifecycleStatus {
    Open,
    Closed,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum RoomMemberRole {
    Host,
    Peer,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum RoomMemberStatus {
    Online,
    Offline,
    Left,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ConnectionPath {
    Direct,
    Relay,
    Failed,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RoomMember {
    pub peer_id: String,
    pub display_name: String,
    pub virtual_ip: Ipv4Addr,
    pub role: RoomMemberRole,
    pub status: RoomMemberStatus,
    pub connection_path: ConnectionPath,
    pub latency_ms: Option<u32>,
    pub packet_loss_percent: Option<f32>,
    pub joined_at_epoch_ms: u128,
    pub last_seen_epoch_ms: Option<u128>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RoomSession {
    pub room_id: String,
    pub room_name: String,
    pub virtual_subnet: Ipv4Subnet,
    pub host_peer_id: String,
    pub status: RoomLifecycleStatus,
    pub members: Vec<RoomMember>,
    pub updated_at_epoch_ms: u128,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RoomSessionSummary {
    pub status: RoomLifecycleStatus,
    pub member_count: usize,
    pub online_count: usize,
    pub left_count: usize,
    pub host_ip: Option<Ipv4Addr>,
    pub next_action: String,
}

impl RoomSession {
    pub fn summary(&self) -> RoomSessionSummary {
        let online_count = self
            .members
            .iter()
            .filter(|member| member.status == RoomMemberStatus::Online)
            .count();
        let left_count = self
            .members
            .iter()
            .filter(|member| member.status == RoomMemberStatus::Left)
            .count();
        let host_ip = self
            .members
            .iter()
            .find(|member| member.role == RoomMemberRole::Host)
            .map(|member| member.virtual_ip);

        RoomSessionSummary {
            status: self.status.clone(),
            member_count: self.members.len(),
            online_count,
            left_count,
            host_ip,
            next_action: if self.status == RoomLifecycleStatus::Closed {
                "Room is closed; stop tunnel and virtual adapter services.".to_owned()
            } else if online_count <= 1 {
                "Copy the invite and wait for peers to join.".to_owned()
            } else if self.members.iter().any(|member| {
                member.status == RoomMemberStatus::Online
                    && member.connection_path == ConnectionPath::Failed
            }) {
                "Open diagnostics for peers whose connection path failed.".to_owned()
            } else {
                "Start the game and try LAN list or Direct IP.".to_owned()
            },
        }
    }
}

pub fn create_room_session(room: &Room, now_epoch_ms: u128) -> Result<RoomSession> {
    let subnet: Ipv4Subnet = room.virtual_subnet.clone().try_into()?;
    Ok(RoomSession {
        room_id: room.room_id.clone(),
        room_name: room.room_name.clone(),
        virtual_subnet: subnet,
        host_peer_id: room.host_peer_id.clone(),
        status: RoomLifecycleStatus::Open,
        members: vec![RoomMember {
            peer_id: room.host_peer_id.clone(),
            display_name: room.host_name.clone(),
            virtual_ip: room.host_ip,
            role: RoomMemberRole::Host,
            status: RoomMemberStatus::Online,
            connection_path: ConnectionPath::Direct,
            latency_ms: Some(0),
            packet_loss_percent: Some(0.0),
            joined_at_epoch_ms: now_epoch_ms,
            last_seen_epoch_ms: Some(now_epoch_ms),
        }],
        updated_at_epoch_ms: now_epoch_ms,
    })
}

pub fn add_room_member(
    session: &mut RoomSession,
    display_name: impl Into<String>,
    peer_id: impl Into<String>,
    peer_ordinal: u32,
    now_epoch_ms: u128,
) -> Result<Ipv4Addr> {
    ensure_open(session)?;
    let peer_id = peer_id.into();
    let virtual_ip = peer_address(session.virtual_subnet, peer_ordinal);
    if session
        .members
        .iter()
        .any(|member| member.peer_id == peer_id)
    {
        return Err(CoreError::InvalidRoomState(format!(
            "peer `{peer_id}` already exists"
        )));
    }
    if session
        .members
        .iter()
        .any(|member| member.virtual_ip == virtual_ip && member.status != RoomMemberStatus::Left)
    {
        return Err(CoreError::InvalidRoomState(format!(
            "virtual IP `{virtual_ip}` is already assigned"
        )));
    }

    session.members.push(RoomMember {
        peer_id,
        display_name: display_name.into(),
        virtual_ip,
        role: RoomMemberRole::Peer,
        status: RoomMemberStatus::Online,
        connection_path: ConnectionPath::Unknown,
        latency_ms: None,
        packet_loss_percent: None,
        joined_at_epoch_ms: now_epoch_ms,
        last_seen_epoch_ms: Some(now_epoch_ms),
    });
    session.updated_at_epoch_ms = now_epoch_ms;
    Ok(virtual_ip)
}

pub fn update_member_connection(
    session: &mut RoomSession,
    peer_id: &str,
    connection_path: ConnectionPath,
    latency_ms: Option<u32>,
    packet_loss_percent: Option<f32>,
    now_epoch_ms: u128,
) -> Result<()> {
    ensure_open(session)?;
    let member = member_mut(session, peer_id)?;
    member.status = if connection_path == ConnectionPath::Failed {
        RoomMemberStatus::Offline
    } else {
        RoomMemberStatus::Online
    };
    member.connection_path = connection_path;
    member.latency_ms = latency_ms;
    member.packet_loss_percent = packet_loss_percent;
    member.last_seen_epoch_ms = Some(now_epoch_ms);
    session.updated_at_epoch_ms = now_epoch_ms;
    Ok(())
}

pub fn mark_member_left(
    session: &mut RoomSession,
    peer_id: &str,
    now_epoch_ms: u128,
) -> Result<()> {
    ensure_open(session)?;
    if peer_id == session.host_peer_id {
        return close_room(session, now_epoch_ms);
    }
    let member = member_mut(session, peer_id)?;
    member.status = RoomMemberStatus::Left;
    member.connection_path = ConnectionPath::Unknown;
    member.last_seen_epoch_ms = Some(now_epoch_ms);
    session.updated_at_epoch_ms = now_epoch_ms;
    Ok(())
}

pub fn close_room(session: &mut RoomSession, now_epoch_ms: u128) -> Result<()> {
    session.status = RoomLifecycleStatus::Closed;
    for member in &mut session.members {
        if member.status != RoomMemberStatus::Left {
            member.status = RoomMemberStatus::Offline;
            member.connection_path = ConnectionPath::Unknown;
            member.last_seen_epoch_ms = Some(now_epoch_ms);
        }
    }
    session.updated_at_epoch_ms = now_epoch_ms;
    Ok(())
}

fn ensure_open(session: &RoomSession) -> Result<()> {
    if session.status == RoomLifecycleStatus::Closed {
        return Err(CoreError::InvalidRoomState(
            "room is already closed".to_owned(),
        ));
    }
    Ok(())
}

fn member_mut<'a>(session: &'a mut RoomSession, peer_id: &str) -> Result<&'a mut RoomMember> {
    session
        .members
        .iter_mut()
        .find(|member| member.peer_id == peer_id)
        .ok_or_else(|| CoreError::InvalidRoomState(format!("unknown peer `{peer_id}`")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::create_room;

    #[test]
    fn room_session_tracks_members_and_summary() {
        let room = create_room("Friday LAN", "Alice", &[]).unwrap();
        let mut session = create_room_session(&room, 100).unwrap();
        let bob_ip = add_room_member(&mut session, "Bob", "peer_bob", 0, 110).unwrap();

        assert_eq!(bob_ip, peer_address(session.virtual_subnet, 0));
        assert_eq!(session.summary().member_count, 2);
        assert_eq!(session.summary().online_count, 2);

        update_member_connection(
            &mut session,
            "peer_bob",
            ConnectionPath::Direct,
            Some(24),
            Some(0.0),
            120,
        )
        .unwrap();

        let bob = session
            .members
            .iter()
            .find(|member| member.peer_id == "peer_bob")
            .unwrap();
        assert_eq!(bob.connection_path, ConnectionPath::Direct);
        assert_eq!(bob.latency_ms, Some(24));
    }

    #[test]
    fn leaving_host_closes_room() {
        let room = create_room("Friday LAN", "Alice", &[]).unwrap();
        let mut session = create_room_session(&room, 100).unwrap();
        add_room_member(&mut session, "Bob", "peer_bob", 0, 110).unwrap();

        mark_member_left(&mut session, &room.host_peer_id, 120).unwrap();

        assert_eq!(session.status, RoomLifecycleStatus::Closed);
        assert!(session
            .members
            .iter()
            .all(|member| member.status != RoomMemberStatus::Online));
        assert!(session.summary().next_action.contains("closed"));
    }

    #[test]
    fn duplicate_active_virtual_ip_is_rejected() {
        let room = create_room("Friday LAN", "Alice", &[]).unwrap();
        let mut session = create_room_session(&room, 100).unwrap();
        add_room_member(&mut session, "Bob", "peer_bob", 0, 110).unwrap();

        let result = add_room_member(&mut session, "Carol", "peer_carol", 0, 120);

        assert!(matches!(result, Err(CoreError::InvalidRoomState(_))));
    }
}
