//! Identity Manager — Ed25519 keypair generation, persistence, and fingerprint

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
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
}

pub struct IdentityManager {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl IdentityManager {
    /// Generate a new Ed25519 keypair
    pub fn generate() -> Result<Self, IdentityError> {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        info!("new identity generated");
        Ok(Self { signing_key, verifying_key })
    }

    /// Return the peer ID as base58-encoded public key (placeholder — will use libp2p PeerId)
    pub fn peer_id_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Fingerprint for human verification (first 8 hex chars, formatted)
    pub fn fingerprint(&self) -> String {
        let bytes = self.verifying_key.to_bytes();
        let hex_str = hex::encode(&bytes[..8]);
        format!(
            "{} {} {} {}",
            &hex_str[0..4].to_uppercase(),
            &hex_str[4..8].to_uppercase(),
            &hex_str[8..12].to_uppercase(),
            &hex_str[12..16].to_uppercase(),
        )
    }

    /// Sign arbitrary data
    pub fn sign(&self, data: &[u8]) -> Signature {
        self.signing_key.sign(data)
    }

    /// Verify a signature against a peer's public key bytes
    pub fn verify(peer_bytes: &[u8; 32], data: &[u8], signature: &Signature) -> Result<(), IdentityError> {
        let verifying_key = VerifyingKey::from_bytes(peer_bytes)
            .map_err(|_| IdentityError::Verify)?;
        verifying_key.verify(data, signature)
            .map_err(|_| IdentityError::Verify)
    }

    /// Export identity to file (encrypted — Phase 1)
    pub fn export(&self, _path: &str, _passphrase: &str) -> Result<(), IdentityError> {
        // TODO: Argon2id + AES-256-GCM
        info!("identity export requested (not yet implemented)");
        Ok(())
    }

    /// Load identity from file (encrypted — Phase 1)
    pub fn load(_path: &str, _passphrase: &str) -> Result<Self, IdentityError> {
        // TODO: Argon2id + AES-256-GCM
        todo!("identity loading not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_identity() {
        let id = IdentityManager::generate().unwrap();
        let fp = id.fingerprint();
        assert_eq!(fp.len(), 19); // "XXXX XXXX XXXX XXXX"
    }

    #[test]
    fn test_sign_and_verify() {
        let id = IdentityManager::generate().unwrap();
        let data = b"hello synthread";
        let sig = id.sign(data);
        let result = IdentityManager::verify(&id.peer_id_bytes(), data, &sig);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_tampered() {
        let id = IdentityManager::generate().unwrap();
        let sig = id.sign(b"original");
        let result = IdentityManager::verify(&id.peer_id_bytes(), b"tampered", &sig);
        assert!(result.is_err());
    }
}
