use crate::invite::decode_invite;
use crate::ip::{host_address, peer_address, Ipv4Subnet};
use crate::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JoinPlan {
    pub room_id: String,
    pub room_name: String,
    pub mode: String,
    pub virtual_subnet: String,
    pub host_peer_id: String,
    pub host_ip: Ipv4Addr,
    pub suggested_local_ip: Ipv4Addr,
    pub next_action: String,
}

pub fn create_join_plan(invite_code: &str, peer_ordinal: u32) -> Result<JoinPlan> {
    let payload = decode_invite(invite_code)?;
    if payload.version != 1 {
        return Err(CoreError::UnsupportedInviteVersion(payload.version));
    }
    let subnet: Ipv4Subnet = payload.virtual_subnet.parse()?;
    Ok(JoinPlan {
        room_id: payload.room_id,
        room_name: payload.room_name,
        mode: payload.mode,
        virtual_subnet: subnet.to_string(),
        host_peer_id: payload.host_peer_id,
        host_ip: host_address(subnet),
        suggested_local_ip: peer_address(subnet, peer_ordinal),
        next_action: "Enable the virtual adapter, assign the suggested IP, then test connectivity with the host.".to_owned(),
    })
}
