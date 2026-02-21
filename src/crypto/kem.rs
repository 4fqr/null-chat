use pqcrypto_kyber::kyber1024;
use pqcrypto_traits::kem::{SecretKey, SharedSecret};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, StaticSecret};
use rand_core::OsRng;
use sha3::{Digest, Sha3_256};
use zeroize::Zeroize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KemError {
    #[error("decapsulation failed")]
    DecapFailed,
    #[error("invalid ciphertext length")]
    InvalidCiphertext,
}

pub struct HybridKemPublicKey {
    pub x25519_pk: X25519PublicKey,
    pub kyber_pk: kyber1024::PublicKey,
}

pub struct HybridKemSecretKey {
    x25519_sk: StaticSecret,
    kyber_sk: kyber1024::SecretKey,
}

pub struct HybridCiphertext {
    pub x25519_pk: X25519PublicKey,
    pub kyber_ct: kyber1024::Ciphertext,
}

pub struct HybridKem;

impl HybridKem {
    pub fn generate_keypair() -> (HybridKemPublicKey, HybridKemSecretKey) {
        let x25519_sk = StaticSecret::random_from_rng(OsRng);
        let x25519_pk = X25519PublicKey::from(&x25519_sk);
        let (kyber_pk, kyber_sk) = kyber1024::keypair();
        (
            HybridKemPublicKey { x25519_pk, kyber_pk },
            HybridKemSecretKey { x25519_sk, kyber_sk },
        )
    }

    pub fn encapsulate(recipient_pk: &HybridKemPublicKey) -> (HybridCiphertext, [u8; 32]) {
        let ephemeral_sk = EphemeralSecret::random_from_rng(OsRng);
        let ephemeral_pk = X25519PublicKey::from(&ephemeral_sk);
        let x25519_shared = ephemeral_sk.diffie_hellman(&recipient_pk.x25519_pk);

        let (kyber_ss, kyber_ct) = kyber1024::encapsulate(&recipient_pk.kyber_pk);

        let combined = Self::combine_shared_secrets(
            x25519_shared.as_bytes(),
            kyber_ss.as_bytes(),
        );

        (
            HybridCiphertext { x25519_pk: ephemeral_pk, kyber_ct },
            combined,
        )
    }

    pub fn decapsulate(
        sk: &HybridKemSecretKey,
        ciphertext: &HybridCiphertext,
    ) -> Result<[u8; 32], KemError> {
        let x25519_shared = sk.x25519_sk.diffie_hellman(&ciphertext.x25519_pk);
        let kyber_ss = kyber1024::decapsulate(&ciphertext.kyber_ct, &sk.kyber_sk);
        Ok(Self::combine_shared_secrets(
            x25519_shared.as_bytes(),
            kyber_ss.as_bytes(),
        ))
    }

    fn combine_shared_secrets(x25519_ss: &[u8], kyber_ss: &[u8]) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(b"NCP-HYBRID-KEM-v1");
        hasher.update(x25519_ss);
        hasher.update(kyber_ss);
        hasher.finalize().into()
    }
}

impl Drop for HybridKemSecretKey {
    fn drop(&mut self) {
        let mut sk_bytes = self.kyber_sk.as_bytes().to_vec();
        sk_bytes.zeroize();
    }
}
