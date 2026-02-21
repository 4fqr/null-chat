use uuid::Uuid;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::crypto::{DoubleRatchetSession, LocalIdentity};
use crate::crypto::kem::HybridKemPublicKey;

#[derive(Error, Debug)]
pub enum NcpError {
    #[error("handshake failed: {0}")]
    HandshakeFailed(String),
    #[error("ratchet error: {0}")]
    RatchetError(#[from] crate::crypto::ratchet::RatchetError),
    #[error("session not established")]
    NoSession,
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NcpEnvelope {
    pub session_id: String,
    pub header: crate::crypto::ratchet::RatchetHeader,
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NcpHandshakeInit {
    pub initiator_identity_pub: [u8; 32],
    pub initiator_kem_pub_x25519: [u8; 32],
    pub initiator_kem_pub_kyber: Vec<u8>,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NcpHandshakeResponse {
    pub responder_identity_pub: [u8; 32],
    pub kem_ciphertext_x25519: [u8; 32],
    pub kem_ciphertext_kyber: Vec<u8>,
    pub ratchet_pub: [u8; 32],
    pub session_id: String,
}

pub struct NcpSession {
    session_id: String,
    local_identity: LocalIdentity,
    ratchet: Option<DoubleRatchetSession>,
}

impl NcpSession {
    pub fn new(local_identity: LocalIdentity) -> Self {
        Self {
            session_id: Uuid::new_v4().to_string(),
            local_identity,
            ratchet: None,
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn local_fingerprint(&self) -> String {
        self.local_identity.fingerprint_hex()
    }

    pub fn build_handshake_init(
        &self,
        kem_pk: &HybridKemPublicKey,
    ) -> NcpHandshakeInit {
        use pqcrypto_traits::kem::PublicKey;
        NcpHandshakeInit {
            initiator_identity_pub: *self.local_identity.verifying_key().as_bytes(),
            initiator_kem_pub_x25519: *kem_pk.x25519_pk.as_bytes(),
            initiator_kem_pub_kyber: kem_pk.kyber_pk.as_bytes().to_vec(),
            session_id: self.session_id.clone(),
        }
    }

    pub fn establish_sender(&mut self, shared_secret: [u8; 32], remote_ratchet_pub: [u8; 32]) {
        self.ratchet = Some(DoubleRatchetSession::init_sender(shared_secret, remote_ratchet_pub));
    }

    pub fn establish_receiver(&mut self, shared_secret: [u8; 32]) -> [u8; 32] {
        let (session, pub_bytes) = DoubleRatchetSession::init_receiver(shared_secret);
        self.ratchet = Some(session);
        pub_bytes
    }

    pub fn send(&mut self, plaintext: &[u8]) -> Result<NcpEnvelope, NcpError> {
        let ratchet = self.ratchet.as_mut().ok_or(NcpError::NoSession)?;
        let (header, ciphertext) = ratchet.ratchet_encrypt(plaintext)?;
        Ok(NcpEnvelope {
            session_id: self.session_id.clone(),
            header,
            ciphertext,
        })
    }

    pub fn receive(&mut self, envelope: &NcpEnvelope) -> Result<Vec<u8>, NcpError> {
        let ratchet = self.ratchet.as_mut().ok_or(NcpError::NoSession)?;
        Ok(ratchet.ratchet_decrypt(&envelope.header, &envelope.ciphertext)?)
    }
}
