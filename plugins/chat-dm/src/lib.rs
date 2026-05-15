//! Chat DM Plugin — encrypted direct messages over libp2p.

pub mod envelope;
pub mod store;

use crate::envelope::MessageEnvelope;
use crate::store::MessageStore;
use ed25519_dalek::{Signer, Verifier};
use std::collections::HashMap;
use std::path::PathBuf;
use synthread_core::plugin::{Plugin, PluginContext};
use synthread_core::security::encryption::E2EE;

pub const PROTOCOL: &str = "/synthread/chat-dm/1.0.0";

pub struct ChatDmPlugin {
    ctx: Option<PluginContext>,
    store: MessageStore,
    /// Cache: peer_id → Ed25519 public key (32 bytes) for signature verification.
    peer_pubkeys: HashMap<String, [u8; 32]>,
    /// Cache: peer_id → X25519 public key (32 bytes) for E2EE encryption.
    peer_x25519_pubkeys: HashMap<String, [u8; 32]>,
}

impl ChatDmPlugin {
    pub fn new() -> Self {
        Self {
            ctx: None,
            store: MessageStore::new(),
            peer_pubkeys: HashMap::new(),
            peer_x25519_pubkeys: HashMap::new(),
        }
    }

    /// Create a plugin with persistent encrypted store.
    pub fn with_persistent_store(store_path: PathBuf, passphrase: String) -> Self {
        Self {
            ctx: None,
            store: MessageStore::persistent(store_path, passphrase),
            peer_pubkeys: HashMap::new(),
            peer_x25519_pubkeys: HashMap::new(),
        }
    }

    /// Cache a peer's public key for E2EE.
    pub fn set_peer_x25519_pubkey(&mut self, peer_id: &str, pubkey: [u8; 32]) {
        self.peer_x25519_pubkeys.insert(peer_id.to_string(), pubkey);
    }

    /// Cache a peer's Ed25519 public key for signature verification.
    pub fn set_peer_ed25519_pubkey(&mut self, peer_id: &str, pubkey: [u8; 32]) {
        self.peer_pubkeys.insert(peer_id.to_string(), pubkey);
    }

    /// Send an encrypted DM to a peer.
    pub fn send_dm(&mut self, to_peer: &str, text: &str) -> Result<String, String> {
        let ctx = self.ctx.as_ref().ok_or("plugin not loaded")?;
        let signing_key = ctx.signing_key_bytes.ok_or("no signing key available")?;

        // Resolve recipient X25519 pubkey
        let recipient_pubkey = self
            .peer_x25519_pubkeys
            .get(to_peer)
            .ok_or_else(|| format!("no X25519 pubkey for {}", to_peer))?;

        // 1. Encrypt with E2EE
        let (ciphertext, ephemeral_pubkey, nonce) =
            E2EE::encrypt(recipient_pubkey, text.as_bytes())?;

        // 2. Create envelope
        let b64 =
            |data: &[u8]| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data);
        let envelope = MessageEnvelope::new(
            &ctx.peer_id,
            to_peer,
            &b64(&ciphertext),
            &b64(&nonce),
            &b64(&ephemeral_pubkey),
            "", // signature filled in below
        );

        // 3. Sign the envelope payload with Ed25519
        let payload_bytes = serde_json::to_vec(&envelope.payload)
            .map_err(|e| format!("serialize payload: {}", e))?;
        let sig = ed25519_dalek::SigningKey::from_keypair_bytes(&signing_key)
            .map_err(|e| format!("invalid signing key: {}", e))?
            .sign(&payload_bytes);
        let envelope = MessageEnvelope {
            signature: b64(&sig.to_bytes()),
            ..envelope
        };

        let msg_id = envelope.id.clone();

        // 4. Serialize and send via core
        let wire_bytes =
            serde_json::to_vec(&envelope).map_err(|e| format!("serialize envelope: {}", e))?;
        ctx.send_message(to_peer, PROTOCOL, wire_bytes)?;

        // 5. Store locally
        self.store.store(to_peer, envelope);

        tracing::info!("DM sent to {} (msg_id={})", to_peer, msg_id);
        Ok(msg_id)
    }

    /// Receive and decrypt an incoming DM.
    pub fn receive_dm(
        &mut self,
        data: &[u8],
        our_x25519_privkey: &[u8; 32],
    ) -> Result<(MessageEnvelope, String), String> {
        let b64_decode = |s: &str| -> Result<Vec<u8>, String> {
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s)
                .map_err(|e| format!("base64 decode: {}", e))
        };

        // 1. Deserialize envelope
        let envelope: MessageEnvelope =
            serde_json::from_slice(data).map_err(|e| format!("deserialize envelope: {}", e))?;

        // 2. Verify Ed25519 signature
        let payload_bytes = serde_json::to_vec(&envelope.payload)
            .map_err(|e| format!("serialize payload for verify: {}", e))?;
        let sig_bytes: [u8; 64] = b64_decode(&envelope.signature)?
            .try_into()
            .map_err(|_| "invalid signature length")?;
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);

        // Resolve sender's Ed25519 pubkey
        let sender_pubkey = self
            .peer_pubkeys
            .get(&envelope.from)
            .copied()
            .unwrap_or([0u8; 32]);

        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&sender_pubkey)
            .map_err(|_| "invalid sender pubkey")?;
        verifying_key
            .verify(&payload_bytes, &sig)
            .map_err(|e| format!("signature verification failed: {}", e))?;

        // 3. Decrypt with X25519 ECDH
        let ephemeral_pubkey_bytes: [u8; 32] = b64_decode(&envelope.payload.ephemeral_pubkey)?
            .try_into()
            .map_err(|_| "invalid ephemeral pubkey length")?;
        let ciphertext = b64_decode(&envelope.payload.body)?;
        let nonce_bytes: [u8; 12] = b64_decode(&envelope.payload.nonce)?
            .try_into()
            .map_err(|_| "invalid nonce length")?;

        let plaintext = E2EE::decrypt(
            our_x25519_privkey,
            &ephemeral_pubkey_bytes,
            &ciphertext,
            &nonce_bytes,
        )?;
        let text = String::from_utf8(plaintext).map_err(|e| format!("utf8: {}", e))?;

        tracing::info!(
            "DM received from {} (msg_id={}): {}",
            envelope.from,
            envelope.id,
            &text[..text.len().min(50)]
        );

        // 4. Store
        self.store.store(&envelope.from, envelope.clone());

        Ok((envelope, text))
    }

    /// Get messages from the local store.
    pub fn get_messages(
        &self,
        peer_id: &str,
        since: Option<&str>,
        limit: usize,
    ) -> Vec<&MessageEnvelope> {
        self.store.get_messages(peer_id, since, limit)
    }

    /// Delete a message by ID.
    pub fn delete_message(&mut self, msg_id: &str) -> bool {
        self.store.delete_by_id(msg_id)
    }

    /// List peers with pending messages.
    pub fn pending_peers(&self) -> Vec<(&str, usize)> {
        self.store.pending_peers()
    }
}

impl Plugin for ChatDmPlugin {
    fn id(&self) -> &str {
        "chat-dm"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["chat/v1".to_string()]
    }

    fn on_load(&mut self, ctx: PluginContext) {
        tracing::info!("Chat DM plugin loaded — peer: {}", ctx.peer_id);
        self.ctx = Some(ctx);
    }

    fn on_unload(&mut self) {
        tracing::info!("Chat DM plugin unloaded");
    }

    fn on_message(&mut self, _peer_id: &str, protocol: &str, _data: &[u8]) {
        if protocol == PROTOCOL {
            tracing::debug!("Chat DM message received — handled via receive_dm()");
        }
    }
}
