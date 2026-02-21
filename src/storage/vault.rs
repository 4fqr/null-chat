use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use heed::{Database, Env, EnvOpenOptions};
use heed::types::Bytes;
use rand::RngCore;
use std::path::Path;
use thiserror::Error;
use zeroize::Zeroize;

const VAULT_VERSION: u8 = 1;
const KDF_MEMORY_COST: u32 = 65536;
const KDF_TIME_COST: u32 = 3;
const KDF_PARALLELISM: u32 = 4;

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("incorrect passphrase or corrupted vault")]
    Decryption,
    #[error("vault not open")]
    NotOpen,
    #[error("serialization failure: {0}")]
    Serialization(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct EncryptedVault {
    env: Option<Env>,
    aes_key: Option<[u8; 32]>,
}

impl EncryptedVault {
    pub fn new() -> Self {
        Self { env: None, aes_key: None }
    }

    pub fn default_path() -> std::path::PathBuf {
        let base = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
        base.join(".local/share/null-chat/vault")
    }

    pub fn is_first_run(vault_path: &Path) -> bool {
        !vault_path.join(".vault_kdf_params").exists()
    }

    pub fn open(&mut self, path: &Path, passphrase: &str) -> Result<(), VaultError> {
        std::fs::create_dir_all(path)?;
        let key_file = path.join(".vault_kdf_params");
        let mut key_salt = [0u8; 32];

        if key_file.exists() {
            let salt_hex = std::fs::read_to_string(&key_file)
                .map_err(|e| VaultError::Io(e))?;
            let salt_bytes = hex::decode(salt_hex.trim())
                .map_err(|_| VaultError::Decryption)?;
            if salt_bytes.len() != 32 {
                return Err(VaultError::Decryption);
            }
            key_salt.copy_from_slice(&salt_bytes);
        } else {
            rand::thread_rng().fill_bytes(&mut key_salt);
            std::fs::write(&key_file, hex::encode(key_salt))?;
        }

        let mut derived = [0u8; 32];
        Argon2::default()
            .hash_password_into(
                passphrase.as_bytes(),
                &key_salt,
                &mut derived,
            )
            .map_err(|_| VaultError::Decryption)?;

        self.aes_key = Some(derived);

        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(256 * 1024 * 1024)
                .max_dbs(16)
                .open(path)
                .map_err(|e| VaultError::Database(e.to_string()))?
        };

        self.env = Some(env);
        tracing::info!("Encrypted vault opened at {:?}", path);
        Ok(())
    }

    pub fn put(&self, db_name: &str, key: &[u8], value: &[u8]) -> Result<(), VaultError> {
        let env = self.env.as_ref().ok_or(VaultError::NotOpen)?;
        let aes_key = self.aes_key.as_ref().ok_or(VaultError::NotOpen)?;

        let ciphertext = self.encrypt_bytes(aes_key, value)?;
        let mut wtxn = env.write_txn().map_err(|e| VaultError::Database(e.to_string()))?;
        let db: Database<Bytes, Bytes> = env
            .create_database(&mut wtxn, Some(db_name))
            .map_err(|e| VaultError::Database(e.to_string()))?;
        db.put(&mut wtxn, key, &ciphertext)
            .map_err(|e| VaultError::Database(e.to_string()))?;
        wtxn.commit().map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn get(&self, db_name: &str, key: &[u8]) -> Result<Option<Vec<u8>>, VaultError> {
        let env = self.env.as_ref().ok_or(VaultError::NotOpen)?;
        let aes_key = self.aes_key.as_ref().ok_or(VaultError::NotOpen)?;

        let rtxn = env.read_txn().map_err(|e| VaultError::Database(e.to_string()))?;
        let db: Database<Bytes, Bytes> = match env
            .open_database(&rtxn, Some(db_name))
            .map_err(|e| VaultError::Database(e.to_string()))?
        {
            Some(db) => db,
            None => return Ok(None),
        };

        match db.get(&rtxn, key).map_err(|e| VaultError::Database(e.to_string()))? {
            Some(ciphertext) => Ok(Some(self.decrypt_bytes(aes_key, ciphertext)?)),
            None => Ok(None),
        }
    }

    pub fn delete(&self, db_name: &str, key: &[u8]) -> Result<(), VaultError> {
        let env = self.env.as_ref().ok_or(VaultError::NotOpen)?;
        let mut wtxn = env.write_txn().map_err(|e| VaultError::Database(e.to_string()))?;
        let db: Option<Database<Bytes, Bytes>> = env
            .open_database(&wtxn, Some(db_name))
            .map_err(|e| VaultError::Database(e.to_string()))?;
        if let Some(db) = db {
            db.delete(&mut wtxn, key)
                .map_err(|e| VaultError::Database(e.to_string()))?;
        }
        wtxn.commit().map_err(|e| VaultError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn close(&mut self) {
        self.env.take();
        if let Some(mut key) = self.aes_key.take() {
            key.zeroize();
        }
    }

    fn encrypt_bytes(&self, key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, VaultError> {
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| VaultError::Decryption)?;
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let mut ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| VaultError::Decryption)?;
        let mut result = nonce_bytes.to_vec();
        result.append(&mut ciphertext);
        Ok(result)
    }

    fn decrypt_bytes(&self, key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, VaultError> {
        if data.len() < 12 {
            return Err(VaultError::Decryption);
        }
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| VaultError::Decryption)?;
        let nonce = Nonce::from_slice(nonce_bytes);
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| VaultError::Decryption)
    }
}

impl Default for EncryptedVault {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for EncryptedVault {
    fn drop(&mut self) {
        self.close();
    }
}
