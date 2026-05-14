//! Visibility Layer — controls what data is exposed to the DHT

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    /// Never published to DHT, only in local store
    Private,
    /// Published to DHT, encrypted for specific peers
    Peers(Vec<String>),
    /// Published to DHT, readable by anyone
    Public,
}

impl Default for Visibility {
    fn default() -> Self {
        Self::Private
    }
}

/// Content hash for referencing stored content
pub type ContentHash = String;

pub struct VisibilityLayer {}

impl VisibilityLayer {
    pub fn new() -> Self {
        Self {}
    }

    /// Store content with visibility rules
    pub fn store(&self, _plugin_id: &str, _content: &[u8], _visibility: Visibility) -> ContentHash {
        todo!()
    }

    /// Retrieve content by hash (checking access)
    pub fn get(&self, _hash: &ContentHash) -> Option<Vec<u8>> {
        todo!()
    }

    /// Check if a peer can access content
    pub fn can_access(&self, _peer_id: &str, _hash: &ContentHash) -> bool {
        todo!()
    }
}
