use crate::ip::{broadcast_address, host_address, subnet_for_room, Ipv4Subnet};
use crate::Result;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Room {
    pub version: u16,
    pub room_id: String,
    pub room_name: String,
    pub host_name: String,
    pub host_peer_id: String,
    pub host_endpoint: Option<String>,
    pub coordination_endpoint: Option<String>,
    pub mode: String,
    pub virtual_subnet: Ipv4SubnetSerde,
    pub join_token: String,
    pub room_key: String,
    pub created_at_epoch_ms: u128,
    pub host_ip: Ipv4Addr,
    pub broadcast_ip: Ipv4Addr,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Ipv4SubnetSerde(pub String);

impl From<Ipv4Subnet> for Ipv4SubnetSerde {
    fn from(value: Ipv4Subnet) -> Self {
        Self(value.to_string())
    }
}

impl TryFrom<Ipv4SubnetSerde> for Ipv4Subnet {
    type Error = crate::CoreError;

    fn try_from(value: Ipv4SubnetSerde) -> crate::Result<Self> {
        value.0.parse()
    }
}

pub fn create_room(
    room_name: impl Into<String>,
    host_name: impl Into<String>,
    local_networks: &[Ipv4Subnet],
) -> Result<Room> {
    let room_id = random_token(8);
    let subnet = subnet_for_room(&room_id, local_networks)?;
    let created_at_epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    Ok(Room {
        version: 1,
        room_id,
        room_name: room_name.into(),
        host_name: host_name.into(),
        host_peer_id: format!("peer_{}", random_token(6)),
        host_endpoint: None,
        coordination_endpoint: None,
        mode: "p2p".to_owned(),
        virtual_subnet: subnet.into(),
        join_token: random_token(18),
        room_key: random_token(32),
        created_at_epoch_ms,
        host_ip: host_address(subnet),
        broadcast_ip: broadcast_address(subnet),
    })
}

fn random_token(bytes: usize) -> String {
    let mut data = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut data);
    URL_SAFE_NO_PAD.encode(data)
}
