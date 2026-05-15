//! Encryption Layer — Noise transport + E2EE message encryption + at-rest storage.
//!
//! Transport: libp2p Noise IK (handled by NetworkLayer)
//! E2EE: X25519 ECDH + ChaCha20-Poly1305 with ephemeral keypair (PFS)
//! At-rest: Argon2id + AES-256-GCM

use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use rand::Rng;
use tracing::info;
use x25519_dalek::{EphemeralSecret, PublicKey};

/// End-to-end encryption: X25519 ECDH + ChaCha20-Poly1305.
///
/// Each message uses an ephemeral keypair for forward secrecy (PFS).
/// The shared secret is derived via X25519 ECDH:
///   shared = X25519(our_ephemeral_privkey, recipient_pubkey)
/// The ChaCha20-Poly1305 key is derived from the shared secret via HKDF-style hash.
pub struct E2EE;

impl E2EE {
    pub fn new() -> Self {
        info!("E2EE layer initialized");
        Self
    }

    /// Encrypt a message for a recipient.
    ///
    /// Returns (ciphertext, ephemeral_pubkey, nonce).
    pub fn encrypt(
        recipient_pubkey: &[u8; 32],
        plaintext: &[u8],
    ) -> Result<(Vec<u8>, [u8; 32], [u8; 12]), String> {
        // Generate ephemeral keypair (PFS)
        let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
        let ephemeral_pubkey = PublicKey::from(&ephemeral_secret);

        // Decode recipient's static public key
        let recipient_pk = PublicKey::from(*recipient_pubkey);

        // X25519 ECDH: shared_secret = X25519(ephemeral_priv, recipient_pub)
        let shared_secret = ephemeral_secret.diffie_hellman(&recipient_pk);

        // Derive symmetric key from shared secret
        let key = derive_symmetric_key(shared_secret.as_bytes());

        // Encrypt with ChaCha20-Poly1305
        let cipher =
            ChaCha20Poly1305::new_from_slice(&key).map_err(|e| format!("key error: {}", e))?;

        let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| format!("encryption error: {}", e))?;

        // ciphertext includes the 16-byte auth tag appended by ChaCha20-Poly1305

        Ok((ciphertext, *ephemeral_pubkey.as_bytes(), nonce_bytes))
    }

    /// Decrypt a message from a sender.
    ///
    /// `our_static_privkey`: our static X25519 private key
    /// `sender_ephemeral_pubkey`: the ephemeral public key sent with the message
    pub fn decrypt(
        our_static_privkey: &[u8; 32],
        sender_ephemeral_pubkey: &[u8; 32],
        ciphertext: &[u8],
        nonce_bytes: &[u8; 12],
    ) -> Result<Vec<u8>, String> {
        // Decode keys
        let static_secret = x25519_dalek::StaticSecret::from(*our_static_privkey);
        let ephemeral_pk = PublicKey::from(*sender_ephemeral_pubkey);

        // X25519 ECDH: shared_secret = X25519(static_priv, ephemeral_pub)
        let shared_secret = static_secret.diffie_hellman(&ephemeral_pk);

        // Derive symmetric key
        let key = derive_symmetric_key(shared_secret.as_bytes());

        // Decrypt with ChaCha20-Poly1305
        let cipher =
            ChaCha20Poly1305::new_from_slice(&key).map_err(|e| format!("key error: {}", e))?;

        let nonce = Nonce::from_slice(nonce_bytes);

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("decryption error (wrong key or corrupted data): {}", e))
    }
}

/// Derive a 256-bit symmetric key from the 32-byte shared secret.
///
/// Uses SHA-256 on the shared secret to produce a uniformly random
/// 256-bit key suitable for ChaCha20-Poly1305.
fn derive_symmetric_key(shared_secret: &[u8; 32]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"synthread-e2ee-v1");
    hasher.update(shared_secret);
    hasher.finalize().into()
}

/// At-rest encryption: Argon2id key derivation + AES-256-GCM.
pub struct AtRestEncryption;

impl AtRestEncryption {
    /// Encrypt data for at-rest storage.
    pub fn encrypt(
        plaintext: &[u8],
        passphrase: &str,
    ) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), String> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce as AesNonce,
        };
        use argon2::Argon2;
        use sha2::{Digest, Sha256};

        // Salt
        let salt: [u8; 32] = rand::thread_rng().gen();

        // Derive key from passphrase
        let hashed_pw = Sha256::digest(passphrase.as_bytes());
        let mut key = [0u8; 32];
        Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(19_456, 2, 1, Some(32))
                .map_err(|e| format!("argon2 params: {}", e))?,
        )
        .hash_password_into(&hashed_pw, &salt, &mut key)
        .map_err(|e| format!("argon2: {}", e))?;

        // Encrypt
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("aes key: {}", e))?;
        let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
        let nonce = AesNonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| format!("aes encrypt: {}", e))?;

        Ok((ciphertext, salt.to_vec(), nonce_bytes.to_vec()))
    }

    /// Decrypt at-rest data.
    pub fn decrypt(
        ciphertext: &[u8],
        passphrase: &str,
        salt: &[u8],
        nonce_bytes: &[u8],
    ) -> Result<Vec<u8>, String> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce as AesNonce,
        };
        use argon2::Argon2;
        use sha2::{Digest, Sha256};

        let salt_arr: [u8; 32] = salt
            .try_into()
            .map_err(|_| "invalid salt length".to_string())?;

        let hashed_pw = Sha256::digest(passphrase.as_bytes());
        let mut key = [0u8; 32];
        Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(19_456, 2, 1, Some(32))
                .map_err(|e| format!("argon2 params: {}", e))?,
        )
        .hash_password_into(&hashed_pw, &salt_arr, &mut key)
        .map_err(|e| format!("argon2: {}", e))?;

        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("aes key: {}", e))?;
        let nonce = AesNonce::from_slice(nonce_bytes);
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("decrypt error (wrong passphrase?): {}", e))
    }
}

/// Transport-layer encryption is handled by libp2p Noise (not a separate struct).
pub struct TransportEncryption;

impl TransportEncryption {
    pub fn new() -> Self {
        Self {}
    }
}

/// Combined encryption layer.
pub struct EncryptionLayer {
    pub transport: TransportEncryption,
    pub e2ee: E2EE,
    pub at_rest: AtRestEncryption,
}

impl EncryptionLayer {
    pub fn new() -> Self {
        info!("Encryption layer initialized");
        Self {
            transport: TransportEncryption::new(),
            e2ee: E2EE::new(),
            at_rest: AtRestEncryption,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_e2ee_encrypt_decrypt() {
        use x25519_dalek::StaticSecret;

        // Alice generates her static keypair
        let alice_secret = StaticSecret::random_from_rng(OsRng);
        let alice_pubkey = *PublicKey::from(&alice_secret).as_bytes();

        // Bob encrypts a message for Alice
        let plaintext = b"hello alice, this is secret!";
        let (ciphertext, ephemeral_pubkey, nonce) =
            E2EE::encrypt(&alice_pubkey, plaintext).unwrap();

        // Alice decrypts using her static private key + Bob's ephemeral pubkey
        let alice_privkey = *alice_secret.as_bytes();
        let decrypted =
            E2EE::decrypt(&alice_privkey, &ephemeral_pubkey, &ciphertext, &nonce).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_e2ee_wrong_recipient_cannot_decrypt() {
        use x25519_dalek::StaticSecret;

        let alice_secret = StaticSecret::random_from_rng(OsRng);
        let alice_pubkey = *PublicKey::from(&alice_secret).as_bytes();

        let eve_secret = StaticSecret::random_from_rng(OsRng);

        let plaintext = b"secret message";
        let (ciphertext, ephemeral_pubkey, nonce) =
            E2EE::encrypt(&alice_pubkey, plaintext).unwrap();

        // Eve tries to decrypt with her own key
        let eve_privkey = *eve_secret.as_bytes();
        let result = E2EE::decrypt(&eve_privkey, &ephemeral_pubkey, &ciphertext, &nonce);
        assert!(result.is_err());
    }

    #[test]
    fn test_at_rest_encrypt_decrypt() {
        let data = b"sensitive data to store on disk";
        let (ciphertext, salt, nonce) = AtRestEncryption::encrypt(data, "my-password").unwrap();

        let decrypted =
            AtRestEncryption::decrypt(&ciphertext, "my-password", &salt, &nonce).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_at_rest_wrong_password() {
        let data = b"top secret";
        let (ciphertext, salt, nonce) = AtRestEncryption::encrypt(data, "correct").unwrap();

        let result = AtRestEncryption::decrypt(&ciphertext, "wrong", &salt, &nonce);
        assert!(result.is_err());
    }
}
