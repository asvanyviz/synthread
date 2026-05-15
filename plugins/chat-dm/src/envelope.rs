//! MessageEnvelope — serialization format for chat messages

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    #[serde(rename = "private")]
    Private,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    #[serde(rename = "type")]
    pub payload_type: String,
    pub body: String,             // base64-encrypted plaintext
    pub nonce: String,            // base64
    pub ephemeral_pubkey: String, // base64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub version: u8,
    pub id: String,
    pub from: String,
    pub to: String,
    pub timestamp: String,
    pub visibility: Visibility,
    pub payload: Payload,
    pub signature: String,
    pub ttl: u64,
}

impl MessageEnvelope {
    pub fn new(
        from: &str,
        to: &str,
        encrypted_body: &str,
        nonce: &str,
        ephemeral_pubkey: &str,
        signature: &str,
    ) -> Self {
        Self {
            version: 1,
            id: Uuid::new_v4().to_string(),
            from: from.to_string(),
            to: to.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            visibility: Visibility::Private,
            payload: Payload {
                payload_type: "text".to_string(),
                body: encrypted_body.to_string(),
                nonce: nonce.to_string(),
                ephemeral_pubkey: ephemeral_pubkey.to_string(),
            },
            signature: signature.to_string(),
            ttl: 604800, // 7 days
        }
    }
}
