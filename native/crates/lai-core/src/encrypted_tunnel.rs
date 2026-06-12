use crate::{CoreError, Result};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Key, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TunnelEnvelopeMetadata {
    pub sequence: u64,
    pub packet_kind: String,
    pub sent_at_ms: u128,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TunnelEnvelope {
    pub version: u16,
    pub algorithm: String,
    pub metadata: TunnelEnvelopeMetadata,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TunnelPayload {
    pub metadata: TunnelEnvelopeMetadata,
    pub plaintext: Vec<u8>,
}

pub fn seal_tunnel_payload(
    shared_secret: &str,
    packet_kind: impl Into<String>,
    sequence: u64,
    sent_at_ms: u128,
    plaintext: &[u8],
) -> Result<TunnelEnvelope> {
    let metadata = TunnelEnvelopeMetadata {
        sequence,
        packet_kind: packet_kind.into(),
        sent_at_ms,
    };
    let aad = metadata_aad(&metadata)?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let cipher = cipher_from_secret(shared_secret);
    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| CoreError::Crypto)?;

    Ok(TunnelEnvelope {
        version: 1,
        algorithm: "chacha20poly1305-sha256-key".to_owned(),
        metadata,
        nonce: STANDARD_NO_PAD.encode(nonce_bytes),
        ciphertext: STANDARD_NO_PAD.encode(ciphertext),
    })
}

pub fn open_tunnel_payload(
    shared_secret: &str,
    envelope: &TunnelEnvelope,
) -> Result<TunnelPayload> {
    if envelope.version != 1 || envelope.algorithm != "chacha20poly1305-sha256-key" {
        return Err(CoreError::Crypto);
    }
    let nonce = STANDARD_NO_PAD
        .decode(&envelope.nonce)
        .map_err(|_| CoreError::Crypto)?;
    let ciphertext = STANDARD_NO_PAD
        .decode(&envelope.ciphertext)
        .map_err(|_| CoreError::Crypto)?;
    if nonce.len() != 12 {
        return Err(CoreError::Crypto);
    }
    let aad = metadata_aad(&envelope.metadata)?;
    let cipher = cipher_from_secret(shared_secret);
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: &aad,
            },
        )
        .map_err(|_| CoreError::Crypto)?;

    Ok(TunnelPayload {
        metadata: envelope.metadata.clone(),
        plaintext,
    })
}

fn cipher_from_secret(shared_secret: &str) -> ChaCha20Poly1305 {
    let digest = Sha256::digest(shared_secret.as_bytes());
    ChaCha20Poly1305::new(Key::from_slice(&digest))
}

fn metadata_aad(metadata: &TunnelEnvelopeMetadata) -> Result<Vec<u8>> {
    serde_json::to_vec(metadata).map_err(|err| CoreError::Serialization(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_payload_opens_with_same_secret() {
        let envelope = seal_tunnel_payload("room-key", "game-udp", 7, 1234, b"hello").unwrap();
        let payload = open_tunnel_payload("room-key", &envelope).unwrap();

        assert_eq!(payload.plaintext, b"hello");
        assert_eq!(payload.metadata.sequence, 7);
        assert_eq!(payload.metadata.packet_kind, "game-udp");
        assert_ne!(envelope.ciphertext, STANDARD_NO_PAD.encode(b"hello"));
    }

    #[test]
    fn sealed_payload_rejects_wrong_secret() {
        let envelope = seal_tunnel_payload("room-key", "game-udp", 7, 1234, b"hello").unwrap();

        assert!(open_tunnel_payload("other-key", &envelope).is_err());
    }
}
