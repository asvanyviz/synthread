//! Message store — encrypted local storage for offline messages.
//!
//! Messages are stored in-memory and persisted to disk with
//! AES-256-GCM encryption (via AtRestEncryption).
//! File format: encrypted JSON at `~/.synthread/chat-store.enc`

use crate::envelope::MessageEnvelope;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use synthread_core::security::encryption::AtRestEncryption;

/// Persisted store format (JSON).
#[derive(Serialize, Deserialize)]
struct StoreData {
    messages: HashMap<String, Vec<MessageEnvelope>>, // peer_id → messages
}

pub struct MessageStore {
    messages: HashMap<String, Vec<MessageEnvelope>>,
    file_path: Option<PathBuf>,
    passphrase: Option<String>,
}

impl MessageStore {
    pub fn new() -> Self {
        Self {
            messages: HashMap::new(),
            file_path: None,
            passphrase: None,
        }
    }

    /// Create a store with disk persistence.
    pub fn persistent(path: PathBuf, passphrase: String) -> Self {
        let mut store = Self {
            messages: HashMap::new(),
            file_path: Some(path),
            passphrase: Some(passphrase),
        };
        store.load_from_disk();
        store
    }

    /// Store a message.
    pub fn store(&mut self, peer_id: &str, msg: MessageEnvelope) {
        self.messages
            .entry(peer_id.to_string())
            .or_default()
            .push(msg);
        self.save_to_disk();
    }

    /// Get messages for a peer, optionally since a timestamp.
    pub fn get_messages(
        &self,
        peer_id: &str,
        since: Option<&str>,
        limit: usize,
    ) -> Vec<&MessageEnvelope> {
        let msgs = self.messages.get(peer_id);
        match msgs {
            Some(list) => {
                let filtered: Vec<&MessageEnvelope> = if let Some(since_ts) = since {
                    list.iter()
                        .filter(|m| m.timestamp.as_str() > since_ts)
                        .collect()
                } else {
                    list.iter().collect()
                };
                let start = if filtered.len() > limit {
                    filtered.len() - limit
                } else {
                    0
                };
                filtered[start..].to_vec()
            }
            None => vec![],
        }
    }

    /// Delete a message by ID.
    pub fn delete(&mut self, peer_id: &str, msg_id: &str) -> bool {
        if let Some(msgs) = self.messages.get_mut(peer_id) {
            let len_before = msgs.len();
            msgs.retain(|m| m.id != msg_id);
            let changed = msgs.len() != len_before;
            if changed {
                self.save_to_disk();
            }
            changed
        } else {
            false
        }
    }

    /// Delete a message without knowing the peer (iterates all peers).
    pub fn delete_by_id(&mut self, msg_id: &str) -> bool {
        let mut deleted = false;
        for msgs in self.messages.values_mut() {
            let len_before = msgs.len();
            msgs.retain(|m| m.id != msg_id);
            if msgs.len() != len_before {
                deleted = true;
            }
        }
        if deleted {
            self.save_to_disk();
        }
        deleted
    }

    /// Count pending messages for a peer.
    pub fn pending_count(&self, peer_id: &str) -> usize {
        self.messages.get(peer_id).map(|m| m.len()).unwrap_or(0)
    }

    /// List peers with pending messages.
    pub fn pending_peers(&self) -> Vec<(&str, usize)> {
        self.messages
            .iter()
            .map(|(k, v)| (k.as_str(), v.len()))
            .collect()
    }

    // ── Persistence ──

    fn save_to_disk(&self) {
        let (path, passphrase) = match (&self.file_path, &self.passphrase) {
            (Some(p), Some(pw)) => (p, pw),
            _ => return,
        };

        let data = StoreData {
            messages: self.messages.clone(),
        };
        let json = match serde_json::to_vec(&data) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("Failed to serialize store: {}", e);
                return;
            }
        };

        match AtRestEncryption::encrypt(&json, passphrase) {
            Ok((ciphertext, salt, nonce)) => {
                let file = PersistedFile {
                    salt,
                    nonce,
                    ciphertext,
                };
                match serde_json::to_vec(&file) {
                    Ok(bytes) => {
                        if let Err(e) = std::fs::write(path, &bytes) {
                            tracing::error!("Failed to write store: {}", e);
                        }
                    }
                    Err(e) => tracing::error!("Failed to serialize persisted file: {}", e),
                }
            }
            Err(e) => tracing::error!("Failed to encrypt store: {}", e),
        }
    }

    fn load_from_disk(&mut self) {
        let (path, passphrase) = match (&self.file_path, &self.passphrase) {
            (Some(p), Some(pw)) => (p, pw),
            _ => return,
        };

        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(_) => return, // File doesn't exist yet — fresh start
        };

        let file: PersistedFile = match serde_json::from_slice(&bytes) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("Failed to deserialize store file: {}", e);
                return;
            }
        };

        match AtRestEncryption::decrypt(&file.ciphertext, passphrase, &file.salt, &file.nonce) {
            Ok(plaintext) => match serde_json::from_slice::<StoreData>(&plaintext) {
                Ok(data) => {
                    self.messages = data.messages;
                    tracing::info!(
                        "Loaded {} peer conversation(s) from disk",
                        self.messages.len()
                    );
                }
                Err(e) => tracing::warn!("Failed to deserialize store data: {}", e),
            },
            Err(e) => tracing::warn!("Failed to decrypt store (wrong passphrase?): {}", e),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PersistedFile {
    salt: Vec<u8>,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}
