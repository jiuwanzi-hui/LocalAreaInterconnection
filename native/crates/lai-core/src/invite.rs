use crate::room::Room;
use crate::{CoreError, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InvitePayload {
    pub version: u16,
    pub room_id: String,
    pub room_name: String,
    pub mode: String,
    pub virtual_subnet: String,
    pub host_peer_id: String,
    pub host_endpoint: Option<String>,
    pub coordination_endpoint: Option<String>,
    pub join_token: String,
    pub created_at_epoch_ms: u128,
}

pub fn create_invite(room: &Room) -> Result<String> {
    let payload = InvitePayload {
        version: room.version,
        room_id: room.room_id.clone(),
        room_name: room.room_name.clone(),
        mode: room.mode.clone(),
        virtual_subnet: room.virtual_subnet.0.clone(),
        host_peer_id: room.host_peer_id.clone(),
        host_endpoint: room.host_endpoint.clone(),
        coordination_endpoint: room.coordination_endpoint.clone(),
        join_token: room.join_token.clone(),
        created_at_epoch_ms: room.created_at_epoch_ms,
    };
    let body =
        serde_json::to_vec(&payload).map_err(|err| CoreError::Serialization(err.to_string()))?;
    let signature = sign(&body, room.room_key.as_bytes())?;
    Ok(format!("{}.{}", URL_SAFE_NO_PAD.encode(body), signature))
}

pub fn decode_invite(invite_code: &str) -> Result<InvitePayload> {
    let (payload, _) = invite_code
        .split_once('.')
        .ok_or(CoreError::InvalidInvite)?;
    let body = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| CoreError::InvalidInvite)?;
    serde_json::from_slice(&body).map_err(|err| CoreError::Serialization(err.to_string()))
}

pub fn verify_invite(invite_code: &str, room_key: &str) -> Result<bool> {
    let (payload, signature) = invite_code
        .split_once('.')
        .ok_or(CoreError::InvalidInvite)?;
    let body = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| CoreError::InvalidInvite)?;
    Ok(sign(&body, room_key.as_bytes())? == signature)
}

fn sign(body: &[u8], key: &[u8]) -> Result<String> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| CoreError::Crypto)?;
    mac.update(body);
    Ok(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}
