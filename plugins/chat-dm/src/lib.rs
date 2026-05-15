//! Chat DM Plugin — encrypted direct messages over libp2p.
//!
//! Protocol: `/synthread/chat-dm/1.0.0`
//!
//! Each message is:
//! 1. Encrypted with X25519 ECDH + ChaCha20-Poly1305 (ephemeral keypair → PFS)
//! 2. Wrapped in a MessageEnvelope
//! 3. Signed with the sender's Ed25519 key
//! 4. Serialized as JSON and sent over the wire

pub mod envelope;
pub mod store;

use crate::envelope::MessageEnvelope;
use crate::store::MessageStore;
use synthread_core::plugin::{Plugin, PluginContext};
use synthread_core::security::encryption::E2EE;

pub const PROTOCOL: &str = "/synthread/chat-dm/1.0.0";

pub struct ChatDmPlugin {
    ctx: Option<PluginContext>,
    store: MessageStore,
}

impl ChatDmPlugin {
    pub fn new() -> Self {
        Self {
            ctx: None,
            store: MessageStore::new(),
        }
    }

    /// Send an encrypted DM to a peer.
    ///
    /// `recipient_pubkey` is the peer's static X25519 public key (32 bytes).
    /// The plugin handles encryption, envelope creation, and sending.
    pub fn send_dm(
        &mut self,
        to_peer: &str,
        recipient_pubkey: &[u8; 32],
        text: &str,
        signer: impl Fn(&[u8]) -> Vec<u8>,
    ) -> Result<String, String> {
        let ctx = self.ctx.as_ref().ok_or("plugin not loaded")?;

        // 1. Encrypt with E2EE
        let (ciphertext, ephemeral_pubkey, nonce) =
            E2EE::encrypt(recipient_pubkey, text.as_bytes())?;

        // 2. Create envelope
        let envelope = MessageEnvelope::new(
            &ctx.peer_id,
            to_peer,
            &base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &ciphertext),
            &base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &nonce),
            &base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &ephemeral_pubkey,
            ),
            "", // signature filled in below
        );

        // 3. Sign the envelope payload
        let payload_bytes = serde_json::to_vec(&envelope.payload)
            .map_err(|e| format!("serialize payload: {}", e))?;
        let sig = signer(&payload_bytes);
        let envelope = MessageEnvelope {
            signature: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &sig),
            ..envelope
        };

        let msg_id = envelope.id.clone();

        // 4. Serialize and send
        let wire_bytes =
            serde_json::to_vec(&envelope).map_err(|e| format!("serialize envelope: {}", e))?;

        ctx.send_message(to_peer, PROTOCOL, wire_bytes)?;

        // 5. Store locally
        self.store.store(to_peer, envelope);

        tracing::info!("DM sent to {} (msg_id={})", to_peer, msg_id);
        Ok(msg_id)
    }

    /// Receive and decrypt an incoming DM.
    ///
    /// `our_static_privkey` is our static X25519 private key for ECDH.
    /// `verifier` checks the Ed25519 signature against the sender's public key.
    pub fn receive_dm(
        &mut self,
        data: &[u8],
        our_static_privkey: &[u8; 32],
        verifier: impl Fn(&[u8; 32], &[u8], &[u8]) -> bool,
    ) -> Result<MessageEnvelope, String> {
        // 1. Deserialize envelope
        let envelope: MessageEnvelope =
            serde_json::from_slice(data).map_err(|e| format!("deserialize envelope: {}", e))?;

        // 2. Verify signature
        let payload_bytes = serde_json::to_vec(&envelope.payload)
            .map_err(|e| format!("serialize payload for verify: {}", e))?;
        let sig_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &envelope.signature,
        )
        .map_err(|e| format!("decode signature: {}", e))?;
        // Convert sender peer_id to pubkey bytes (placeholder — needs IdentityManager)
        let sender_bytes = [0u8; 32]; // TODO: resolve peer_id → pubkey
        if !verifier(&sender_bytes, &payload_bytes, &sig_bytes) {
            return Err("invalid signature".to_string());
        }

        // 3. Decrypt
        let ephemeral_pubkey_bytes: [u8; 32] = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &envelope.payload.ephemeral_pubkey,
        )
        .map_err(|e| format!("decode ephemeral pubkey: {}", e))?
        .try_into()
        .map_err(|_| "invalid ephemeral pubkey length".to_string())?;

        let ciphertext = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &envelope.payload.body,
        )
        .map_err(|e| format!("decode ciphertext: {}", e))?;

        let nonce_bytes: [u8; 12] = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &envelope.payload.nonce,
        )
        .map_err(|e| format!("decode nonce: {}", e))?
        .try_into()
        .map_err(|_| "invalid nonce length".to_string())?;

        let plaintext = E2EE::decrypt(
            our_static_privkey,
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

        Ok(envelope)
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

    fn on_message(&mut self, peer_id: &str, protocol: &str, data: &[u8]) {
        if protocol == PROTOCOL {
            tracing::debug!("Chat DM message from {}", peer_id);
            // Decryption is deferred — handled by the node which has the keys
        }
    }
}
