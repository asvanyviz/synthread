//! Identity Manager — Ed25519 keypair generation, persistence, and fingerprint
//!
//! Identity files are encrypted with Argon2id (key derivation) + AES-256-GCM.
//! File format (JSON):
//! ```json
//! {
//!   "version": 1,
//!   "salt": "<base64>",
//!   "nonce": "<base64>",
//!   "ciphertext": "<base64>"
//! }
//! ```

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce as AesNonce,
};
use argon2::Argon2;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum IdentityError {
    #[error("key generation failed: {0}")]
    KeyGen(String),
    #[error("signing failed: {0}")]
    Sign(String),
    #[error("verification failed")]
    Verify,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("encryption error: {0}")]
    Encryption(String),
    #[error("decryption error: {0}")]
    Decryption(String),
    #[error("invalid identity file format: {0}")]
    InvalidFormat(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// On-disk format for an encrypted identity file.
#[derive(Serialize, Deserialize)]
struct IdentityFile {
    version: u8,
    salt: String,       // base64
    nonce: String,      // base64
    ciphertext: String, // base64
}

const SALT_LEN: usize = 32;
const ARGON2_MEMORY: u32 = 19_456; // 19 MiB
const ARGON2_ITERATIONS: u32 = 2;
const ARGON2_PARALLELISM: u32 = 1;

pub struct IdentityManager {
    signing_key: SigningKey,
}

impl IdentityManager {
    /// Generate a new Ed25519 keypair.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        info!("new identity generated");
        Self { signing_key }
    }

    /// Return the raw public key bytes (32 bytes).
    pub fn peer_id_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// Return a libp2p-compatible PeerId from the public key.
    pub fn peer_id(&self) -> libp2p::PeerId {
        let pk = libp2p::identity::ed25519::PublicKey::try_from_bytes(&self.peer_id_bytes())
            .expect("valid ed25519 public key");
        let identity_pk = libp2p::identity::PublicKey::from(pk);
        libp2p::PeerId::from(identity_pk)
    }

    /// Fingerprint for human verification.
    /// Format: "8A7F 3B2C 9D1E ..."
    pub fn fingerprint(&self) -> String {
        let bytes = self.peer_id_bytes();
        let hex_str = hex::encode(&bytes[..8]);
        format!(
            "{} {} {} {}",
            &hex_str[0..4].to_uppercase(),
            &hex_str[4..8].to_uppercase(),
            &hex_str[8..12].to_uppercase(),
            &hex_str[12..16].to_uppercase(),
        )
    }

    /// Sign arbitrary data.
    pub fn sign(&self, data: &[u8]) -> Signature {
        self.signing_key.sign(data)
    }

    /// Verify a signature against peer public key bytes.
    pub fn verify(
        peer_bytes: &[u8; 32],
        data: &[u8],
        sig: &Signature,
    ) -> Result<(), IdentityError> {
        let vk = VerifyingKey::from_bytes(peer_bytes).map_err(|_| IdentityError::Verify)?;
        vk.verify(data, sig).map_err(|_| IdentityError::Verify)
    }

    /// Get the signing key bytes (for internal use — transport layer, etc.)
    pub fn to_keypair_bytes(&self) -> [u8; 64] {
        self.signing_key.to_keypair_bytes()
    }

    /// Export identity to an encrypted file (Argon2id + AES-256-GCM).
    pub fn export(&self, path: &str, passphrase: &str) -> Result<(), IdentityError> {
        let keypair = self.signing_key.to_keypair_bytes(); // 64 bytes
        let salt: [u8; SALT_LEN] = rand::Rng::gen(&mut OsRng);
        let encryption_key = derive_key(passphrase, &salt)?;
        let cipher = Aes256Gcm::new_from_slice(&encryption_key)
            .map_err(|e| IdentityError::Encryption(e.to_string()))?;
        let nonce_bytes: [u8; 12] = rand::Rng::gen(&mut OsRng);
        let nonce = AesNonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, keypair.as_ref())
            .map_err(|e| IdentityError::Encryption(e.to_string()))?;

        // AES-GCM appends its 16-byte authentication tag automatically;
        // we store the combined ciphertext+tag.

        let file = IdentityFile {
            version: 1,
            salt: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &salt),
            nonce: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &nonce_bytes),
            ciphertext: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &ciphertext,
            ),
        };

        let serialized =
            serde_json::to_string_pretty(&file).map_err(IdentityError::Serialization)?;
        std::fs::write(path, &serialized)?;
        info!("identity exported to {}", path);
        Ok(())
    }

    /// Load identity from an encrypted file (Argon2id + AES-256-GCM).
    pub fn load(path: &str, passphrase: &str) -> Result<Self, IdentityError> {
        let data = std::fs::read_to_string(path)?;
        let file: IdentityFile = serde_json::from_str(&data)?;

        if file.version != 1 {
            return Err(IdentityError::InvalidFormat(format!(
                "unsupported identity file version: {}",
                file.version
            )));
        }

        let salt: Vec<u8> =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &file.salt)
                .map_err(|e| IdentityError::InvalidFormat(format!("invalid salt base64: {}", e)))?;

        let nonce_bytes: Vec<u8> =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &file.nonce)
                .map_err(|e| {
                    IdentityError::InvalidFormat(format!("invalid nonce base64: {}", e))
                })?;

        let ciphertext: Vec<u8> =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &file.ciphertext)
                .map_err(|e| {
                    IdentityError::InvalidFormat(format!("invalid ciphertext base64: {}", e))
                })?;

        if salt.len() != SALT_LEN {
            return Err(IdentityError::InvalidFormat("invalid salt length".into()));
        }
        if nonce_bytes.len() != 12 {
            return Err(IdentityError::InvalidFormat("invalid nonce length".into()));
        }

        let salt_arr: [u8; SALT_LEN] = salt.try_into().unwrap();
        let encryption_key = derive_key(passphrase, &salt_arr)?;
        let cipher = Aes256Gcm::new_from_slice(&encryption_key)
            .map_err(|e| IdentityError::Decryption(e.to_string()))?;
        let nonce = AesNonce::from_slice(&nonce_bytes);
        let keypair_bytes = cipher.decrypt(nonce, ciphertext.as_ref()).map_err(|e| {
            IdentityError::Decryption(format!("wrong passphrase or corrupted file: {}", e))
        })?;

        if keypair_bytes.len() != 64 {
            return Err(IdentityError::InvalidFormat(
                "decrypted keypair is not 64 bytes".into(),
            ));
        }

        let signing_key = SigningKey::from_keypair_bytes(&keypair_bytes[..].try_into().unwrap())
            .map_err(|e| IdentityError::InvalidFormat(format!("invalid keypair bytes: {}", e)))?;

        info!("identity loaded from {}", path);
        Ok(Self { signing_key })
    }

    /// Check if an identity file exists at the given path.
    pub fn exists(path: &str) -> bool {
        Path::new(path).exists()
    }
}

/// Derive a 256-bit AES key from a passphrase using Argon2id.
fn derive_key(passphrase: &str, salt: &[u8; SALT_LEN]) -> Result<[u8; 32], IdentityError> {
    // First hash the passphrase with SHA-256 to get a consistent-length input
    let hashed = Sha256::digest(passphrase.as_bytes());

    let mut key = [0u8; 32];
    Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(
            ARGON2_MEMORY,
            ARGON2_ITERATIONS,
            ARGON2_PARALLELISM,
            Some(32),
        )
        .map_err(|e| IdentityError::Encryption(e.to_string()))?,
    )
    .hash_password_into(&hashed, salt, &mut key)
    .map_err(|e| IdentityError::Encryption(e.to_string()))?;

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_identity() {
        let id = IdentityManager::generate();
        let fp = id.fingerprint();
        assert_eq!(fp.len(), 19); // "XXXX XXXX XXXX XXXX"
        assert_eq!(id.peer_id_bytes().len(), 32);
    }

    #[test]
    fn test_sign_and_verify() {
        let id = IdentityManager::generate();
        let data = b"hello synthread";
        let sig = id.sign(data);
        IdentityManager::verify(&id.peer_id_bytes(), data, &sig).unwrap();
    }

    #[test]
    fn test_verify_tampered() {
        let id = IdentityManager::generate();
        let sig = id.sign(b"original");
        let result = IdentityManager::verify(&id.peer_id_bytes(), b"tampered", &sig);
        assert!(result.is_err());
    }

    #[test]
    fn test_export_and_load() {
        let id = IdentityManager::generate();
        let fp_before = id.fingerprint();
        let pk_before = id.peer_id_bytes();

        let tmp = "/tmp/synthread-test-identity.enc";
        id.export(tmp, "correct passphrase").unwrap();

        // Load with correct passphrase
        let loaded = IdentityManager::load(tmp, "correct passphrase").unwrap();
        assert_eq!(loaded.fingerprint(), fp_before);
        assert_eq!(loaded.peer_id_bytes(), pk_before);

        // Wrong passphrase should fail
        let err = IdentityManager::load(tmp, "wrong password");
        assert!(err.is_err());

        // Cleanup
        std::fs::remove_file(tmp).ok();
    }

    #[test]
    fn test_peer_id_is_valid() {
        let id = IdentityManager::generate();
        let peer_id = id.peer_id();
        // Verify it round-trips
        let peer_id_str = peer_id.to_base58();
        assert!(!peer_id_str.is_empty());
    }
}
