//! Encryption Layer — Noise transport + E2EE message encryption

use tracing::info;

/// Transport-layer encryption: libp2p Noise IK handshake + session keys
pub struct TransportEncryption {
    // Noise IK via libp2p-noise
}

impl TransportEncryption {
    pub fn new() -> Self {
        Self {}
    }
}

/// End-to-end encryption: X25519 ECDH + ChaCha20-Poly1305 with PFS
pub struct E2EE {
    // Ephemeral keypair per message for forward secrecy
}

impl E2EE {
    pub fn new() -> Self {
        Self {}
    }

    /// Encrypt a message for a recipient
    pub fn encrypt(&self, _recipient_pubkey: &[u8], _plaintext: &[u8]) -> Result<Vec<u8>, String> {
        todo!("E2EE encrypt")
    }

    /// Decrypt a message from a sender
    pub fn decrypt(
        &self,
        _our_privkey: &[u8],
        _sender_ephemeral_pubkey: &[u8],
        _ciphertext: &[u8],
        _nonce: &[u8],
    ) -> Result<Vec<u8>, String> {
        todo!("E2EE decrypt")
    }
}

/// At-rest encryption: Argon2id key derivation + AES-256-GCM (SQLCipher)
pub struct AtRestEncryption;

impl AtRestEncryption {
    pub fn encrypt(&self, _plaintext: &[u8], _passphrase: &str) -> Result<Vec<u8>, String> {
        todo!("At-rest encrypt")
    }

    pub fn decrypt(&self, _ciphertext: &[u8], _passphrase: &str) -> Result<Vec<u8>, String> {
        todo!("At-rest decrypt")
    }
}

pub struct EncryptionLayer {
    pub transport: TransportEncryption,
    pub e2ee: E2EE,
    pub at_rest: AtRestEncryption,
}

impl EncryptionLayer {
    pub fn new() -> Self {
        info!("Encryption layer initialized (stubs)");
        Self {
            transport: TransportEncryption::new(),
            e2ee: E2EE::new(),
            at_rest: AtRestEncryption,
        }
    }
}
