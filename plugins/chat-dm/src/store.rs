//! Message store — encrypted local SQLite storage for offline messages

use std::collections::HashMap;
use crate::envelope::MessageEnvelope;

pub struct MessageStore {
    // In-memory store for Phase 0; SQLCipher for Phase 2
    messages: HashMap<String, Vec<MessageEnvelope>>, // peer_id → messages
}

impl MessageStore {
    pub fn new() -> Self {
        Self { messages: HashMap::new() }
    }

    pub fn store(&mut self, peer_id: &str, msg: MessageEnvelope) {
        self.messages.entry(peer_id.to_string()).or_default().push(msg);
    }

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
                    list.iter().filter(|m| m.timestamp.as_str() > since_ts).collect()
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

    pub fn delete(&mut self, peer_id: &str, msg_id: &str) -> bool {
        if let Some(msgs) = self.messages.get_mut(peer_id) {
            let len_before = msgs.len();
            msgs.retain(|m| m.id != msg_id);
            msgs.len() != len_before
        } else {
            false
        }
    }

    pub fn pending_count(&self, peer_id: &str) -> usize {
        self.messages.get(peer_id).map(|m| m.len()).unwrap_or(0)
    }
}
