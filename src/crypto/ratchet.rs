use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};
use rand_core::OsRng;
use std::collections::HashMap;
use thiserror::Error;

use crate::crypto::kdf::{kdf_rk, ChainKey, MessageKey, RootKey, derive_aead_keys};

const MAX_SKIP: u32 = 1000;

#[derive(Error, Debug)]
pub enum RatchetError {
    #[error("too many skipped messages")]
    TooManySkippedMessages,
    #[error("decryption failed")]
    DecryptionFailed,
    #[error("message key not found for skipped message")]
    SkippedKeyNotFound,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RatchetHeader {
    pub dh_public: [u8; 32],
    pub prev_chain_length: u32,
    pub message_number: u32,
}

pub struct DoubleRatchetSession {
    dh_self: StaticSecret,
    dh_self_pub: X25519PublicKey,
    dh_remote: Option<X25519PublicKey>,
    root_key: RootKey,
    sending_chain: Option<ChainKey>,
    receiving_chain: Option<ChainKey>,
    send_count: u32,
    recv_count: u32,
    prev_send_count: u32,
    skipped_message_keys: HashMap<([u8; 32], u32), [u8; 32]>,
}

impl DoubleRatchetSession {
    pub fn init_sender(shared_secret: [u8; 32], remote_pub: [u8; 32]) -> Self {
        let dh_self = StaticSecret::random_from_rng(OsRng);
        let dh_self_pub = X25519PublicKey::from(&dh_self);
        let dh_remote = X25519PublicKey::from(remote_pub);
        let dh_output = dh_self.diffie_hellman(&dh_remote);

        let (root_key, sending_chain, _header_key) =
            kdf_rk(&RootKey(shared_secret), dh_output.as_bytes());

        Self {
            dh_self,
            dh_self_pub,
            dh_remote: Some(dh_remote),
            root_key,
            sending_chain: Some(sending_chain),
            receiving_chain: None,
            send_count: 0,
            recv_count: 0,
            prev_send_count: 0,
            skipped_message_keys: HashMap::new(),
        }
    }

    pub fn init_receiver(shared_secret: [u8; 32]) -> (Self, [u8; 32]) {
        let dh_self = StaticSecret::random_from_rng(OsRng);
        let dh_self_pub = X25519PublicKey::from(&dh_self);
        let pub_bytes = *dh_self_pub.as_bytes();

        let session = Self {
            dh_self,
            dh_self_pub,
            dh_remote: None,
            root_key: RootKey(shared_secret),
            sending_chain: None,
            receiving_chain: None,
            send_count: 0,
            recv_count: 0,
            prev_send_count: 0,
            skipped_message_keys: HashMap::new(),
        };

        (session, pub_bytes)
    }

    pub fn ratchet_encrypt(&mut self, plaintext: &[u8]) -> Result<(RatchetHeader, Vec<u8>), RatchetError> {
        let chain = self.sending_chain.take().unwrap_or_else(|| {
            let dh_output = self.dh_self.diffie_hellman(
                self.dh_remote.as_ref().expect("remote public key required for send"),
            );
            let (rk, ck, _) = kdf_rk(&self.root_key, dh_output.as_bytes());
            self.root_key = rk;
            ck
        });

        let (next_chain, message_key) = chain.advance();
        self.sending_chain = Some(next_chain);

        let header = RatchetHeader {
            dh_public: *self.dh_self_pub.as_bytes(),
            prev_chain_length: self.prev_send_count,
            message_number: self.send_count,
        };
        self.send_count += 1;

        let ciphertext = encrypt_with_message_key(&message_key, plaintext, &header)?;
        Ok((header, ciphertext))
    }

    pub fn ratchet_decrypt(
        &mut self,
        header: &RatchetHeader,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, RatchetError> {
        let skip_key = (header.dh_public, header.message_number);
        if let Some(mk_bytes) = self.skipped_message_keys.remove(&skip_key) {
            return decrypt_with_message_key_bytes(&mk_bytes, ciphertext, header);
        }

        let remote_pub = X25519PublicKey::from(header.dh_public);
        let is_new_ratchet = self
            .dh_remote
            .as_ref()
            .map(|p| p.as_bytes() != remote_pub.as_bytes())
            .unwrap_or(true);

        if is_new_ratchet {
            self.skip_message_keys(header.prev_chain_length)?;
            self.perform_dh_ratchet(remote_pub);
        }

        self.skip_message_keys(header.message_number)?;

        let chain = self
            .receiving_chain
            .take()
            .ok_or(RatchetError::DecryptionFailed)?;
        let (next_chain, message_key) = chain.advance();
        self.receiving_chain = Some(next_chain);
        self.recv_count += 1;

        decrypt_with_message_key(&message_key, ciphertext, header)
    }

    fn skip_message_keys(&mut self, until: u32) -> Result<(), RatchetError> {
        if until.saturating_sub(self.recv_count) > MAX_SKIP {
            return Err(RatchetError::TooManySkippedMessages);
        }
        while self.recv_count < until {
            let chain = self
                .receiving_chain
                .take()
                .ok_or(RatchetError::DecryptionFailed)?;
            let (next_chain, mk) = chain.advance();
            self.receiving_chain = Some(next_chain);
            self.skipped_message_keys
                .insert((self.dh_remote.unwrap().as_bytes().clone(), self.recv_count), mk.0);
            self.recv_count += 1;
        }
        Ok(())
    }

    fn perform_dh_ratchet(&mut self, remote_pub: X25519PublicKey) {
        self.prev_send_count = self.send_count;
        self.send_count = 0;
        self.recv_count = 0;
        self.dh_remote = Some(remote_pub);

        let dh_recv = self.dh_self.diffie_hellman(&remote_pub);
        let (rk, receiving_chain, _) = kdf_rk(&self.root_key, dh_recv.as_bytes());
        self.root_key = rk;
        self.receiving_chain = Some(receiving_chain);

        let new_dh = StaticSecret::random_from_rng(OsRng);
        let new_pub = X25519PublicKey::from(&new_dh);
        let dh_send = new_dh.diffie_hellman(&remote_pub);
        let (rk2, sending_chain, _) = kdf_rk(&self.root_key, dh_send.as_bytes());
        self.root_key = rk2;
        self.sending_chain = Some(sending_chain);
        self.dh_self = new_dh;
        self.dh_self_pub = new_pub;
    }
}

fn associated_data(header: &RatchetHeader) -> Vec<u8> {
    let mut ad = Vec::with_capacity(40);
    ad.extend_from_slice(&header.dh_public);
    ad.extend_from_slice(&header.prev_chain_length.to_le_bytes());
    ad.extend_from_slice(&header.message_number.to_le_bytes());
    ad
}

fn encrypt_with_message_key(
    mk: &MessageKey,
    plaintext: &[u8],
    header: &RatchetHeader,
) -> Result<Vec<u8>, RatchetError> {
    let (enc_key, nonce_bytes) = derive_aead_keys(mk);
    let cipher = ChaCha20Poly1305::new_from_slice(&enc_key).map_err(|_| RatchetError::DecryptionFailed)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let payload = chacha20poly1305::aead::Payload {
        msg: plaintext,
        aad: &associated_data(header),
    };
    cipher.encrypt(nonce, payload).map_err(|_| RatchetError::DecryptionFailed)
}

fn decrypt_with_message_key(
    mk: &MessageKey,
    ciphertext: &[u8],
    header: &RatchetHeader,
) -> Result<Vec<u8>, RatchetError> {
    let (enc_key, nonce_bytes) = derive_aead_keys(mk);
    decrypt_inner(&enc_key, &nonce_bytes, ciphertext, header)
}

fn decrypt_with_message_key_bytes(
    mk_bytes: &[u8; 32],
    ciphertext: &[u8],
    header: &RatchetHeader,
) -> Result<Vec<u8>, RatchetError> {
    let mk = MessageKey(*mk_bytes);
    let (enc_key, nonce_bytes) = derive_aead_keys(&mk);
    decrypt_inner(&enc_key, &nonce_bytes, ciphertext, header)
}

fn decrypt_inner(
    enc_key: &[u8; 32],
    nonce_bytes: &[u8; 12],
    ciphertext: &[u8],
    header: &RatchetHeader,
) -> Result<Vec<u8>, RatchetError> {
    let cipher = ChaCha20Poly1305::new_from_slice(enc_key).map_err(|_| RatchetError::DecryptionFailed)?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let payload = chacha20poly1305::aead::Payload {
        msg: ciphertext,
        aad: &associated_data(header),
    };
    cipher.decrypt(nonce, payload).map_err(|_| RatchetError::DecryptionFailed)
}


