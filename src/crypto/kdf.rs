use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha3::Sha3_256;
use zeroize::Zeroize;

type HmacSha3 = Hmac<Sha3_256>;

pub struct ChainKey(pub [u8; 32]);
pub struct MessageKey(pub [u8; 32]);
pub struct RootKey(pub [u8; 32]);

impl ChainKey {
    pub fn advance(&self) -> (ChainKey, MessageKey) {
        let mut mk_bytes = [0u8; 32];
        let mut ck_bytes = [0u8; 32];

        let mut mac = HmacSha3::new_from_slice(&self.0).expect("HMAC accepts any key size");
        mac.update(b"\x01");
        mk_bytes.copy_from_slice(&mac.finalize().into_bytes());

        let mut mac = HmacSha3::new_from_slice(&self.0).expect("HMAC accepts any key size");
        mac.update(b"\x02");
        ck_bytes.copy_from_slice(&mac.finalize().into_bytes());

        (ChainKey(ck_bytes), MessageKey(mk_bytes))
    }
}

pub fn kdf_rk(
    root_key: &RootKey,
    dh_output: &[u8; 32],
) -> (RootKey, ChainKey, [u8; 32]) {
    let hkdf = Hkdf::<Sha3_256>::new(Some(&root_key.0), dh_output);
    let mut out = [0u8; 96];
    hkdf.expand(b"NCP-RATCHET-v1", &mut out)
        .expect("96 bytes is valid HKDF output length");

    let mut new_rk = [0u8; 32];
    let mut new_ck = [0u8; 32];
    let mut header_key = [0u8; 32];
    new_rk.copy_from_slice(&out[0..32]);
    new_ck.copy_from_slice(&out[32..64]);
    header_key.copy_from_slice(&out[64..96]);

    (RootKey(new_rk), ChainKey(new_ck), header_key)
}

pub fn derive_aead_keys(message_key: &MessageKey) -> ([u8; 32], [u8; 12]) {
    let hkdf = Hkdf::<Sha3_256>::new(None, &message_key.0);
    let mut enc_key = [0u8; 32];
    let mut nonce = [0u8; 12];
    hkdf.expand(b"NCP-ENC-KEY", &mut enc_key).unwrap();
    hkdf.expand(b"NCP-NONCE", &mut nonce).unwrap();
    (enc_key, nonce)
}

impl Drop for ChainKey {
    fn drop(&mut self) { self.0.zeroize(); }
}
impl Drop for MessageKey {
    fn drop(&mut self) { self.0.zeroize(); }
}
impl Drop for RootKey {
    fn drop(&mut self) { self.0.zeroize(); }
}
