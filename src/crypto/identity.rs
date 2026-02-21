use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand_core::OsRng;
use sha3::{Digest, Sha3_256};
use thiserror::Error;
use zeroize::Zeroize;

#[derive(Error, Debug)]
pub enum IdentityError {
    #[error("signature verification failed")]
    VerificationFailed,
    #[error("invalid key material: {0}")]
    InvalidKey(String),
}

pub struct LocalIdentity {
    signing_key: SigningKey,
    fingerprint: [u8; 32],
}

impl LocalIdentity {
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let fingerprint = Self::derive_fingerprint(signing_key.verifying_key().as_bytes());
        Self { signing_key, fingerprint }
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, IdentityError> {
        let signing_key = SigningKey::from_bytes(bytes);
        let fingerprint = Self::derive_fingerprint(signing_key.verifying_key().as_bytes());
        Ok(Self { signing_key, fingerprint })
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    pub fn fingerprint_hex(&self) -> String {
        hex::encode(self.fingerprint)
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    pub fn verify(
        verifying_key: &VerifyingKey,
        message: &[u8],
        signature: &Signature,
    ) -> Result<(), IdentityError> {
        verifying_key
            .verify(message, signature)
            .map_err(|_| IdentityError::VerificationFailed)
    }

    pub fn signing_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    fn derive_fingerprint(public_key_bytes: &[u8]) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(public_key_bytes);
        hasher.finalize().into()
    }
}

impl Drop for LocalIdentity {
    fn drop(&mut self) {
        let mut key_bytes = self.signing_key.to_bytes();
        key_bytes.zeroize();
        self.fingerprint.zeroize();
    }
}
