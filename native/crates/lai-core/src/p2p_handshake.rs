use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct P2pHandshakeHello {
    pub version: u16,
    pub room_id: String,
    pub peer_id: String,
    pub virtual_ip: Ipv4Addr,
    pub listen_endpoint: String,
    pub nonce: String,
    pub sent_at_ms: u128,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct P2pHandshakeAck {
    pub version: u16,
    pub room_id: String,
    pub responder_peer_id: String,
    pub accepted: bool,
    pub observed_endpoint: String,
    pub nonce: String,
    pub sent_at_ms: u128,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct P2pHandshakeConfirm {
    pub version: u16,
    pub room_id: String,
    pub confirmer_peer_id: String,
    pub responder_peer_id: String,
    pub nonce: String,
    pub sent_at_ms: u128,
}

pub fn create_p2p_handshake_hello(
    room_id: impl Into<String>,
    peer_id: impl Into<String>,
    virtual_ip: Ipv4Addr,
    listen_endpoint: impl Into<String>,
    nonce: impl Into<String>,
    sent_at_ms: u128,
) -> P2pHandshakeHello {
    P2pHandshakeHello {
        version: 1,
        room_id: room_id.into(),
        peer_id: peer_id.into(),
        virtual_ip,
        listen_endpoint: listen_endpoint.into(),
        nonce: nonce.into(),
        sent_at_ms,
    }
}

pub fn create_p2p_handshake_ack(
    hello: &P2pHandshakeHello,
    responder_peer_id: impl Into<String>,
    observed_endpoint: impl Into<String>,
    sent_at_ms: u128,
) -> P2pHandshakeAck {
    P2pHandshakeAck {
        version: 1,
        room_id: hello.room_id.clone(),
        responder_peer_id: responder_peer_id.into(),
        accepted: true,
        observed_endpoint: observed_endpoint.into(),
        nonce: hello.nonce.clone(),
        sent_at_ms,
        message: "encrypted P2P handshake accepted".to_owned(),
    }
}

pub fn create_p2p_handshake_confirm(
    room_id: impl Into<String>,
    confirmer_peer_id: impl Into<String>,
    responder_peer_id: impl Into<String>,
    nonce: impl Into<String>,
    sent_at_ms: u128,
) -> P2pHandshakeConfirm {
    P2pHandshakeConfirm {
        version: 1,
        room_id: room_id.into(),
        confirmer_peer_id: confirmer_peer_id.into(),
        responder_peer_id: responder_peer_id.into(),
        nonce: nonce.into(),
        sent_at_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p2p_handshake_ack_echoes_room_and_nonce() {
        let hello = create_p2p_handshake_hello(
            "room_1",
            "peer_a",
            "10.77.12.2".parse().unwrap(),
            "127.0.0.1:39090",
            "nonce-1",
            100,
        );
        let ack = create_p2p_handshake_ack(&hello, "peer_b", "127.0.0.1:50000", 120);

        assert_eq!(ack.version, 1);
        assert_eq!(ack.room_id, "room_1");
        assert_eq!(ack.nonce, "nonce-1");
        assert!(ack.accepted);
        assert_eq!(ack.responder_peer_id, "peer_b");

        let confirm = create_p2p_handshake_confirm(
            &ack.room_id,
            "peer_a",
            &ack.responder_peer_id,
            &ack.nonce,
            130,
        );
        assert_eq!(confirm.version, 1);
        assert_eq!(confirm.room_id, "room_1");
        assert_eq!(confirm.confirmer_peer_id, "peer_a");
        assert_eq!(confirm.responder_peer_id, "peer_b");
        assert_eq!(confirm.nonce, "nonce-1");
    }
}
