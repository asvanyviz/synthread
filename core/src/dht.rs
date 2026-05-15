//! DHT Engine — Kademlia DHT via libp2p-kad.
//!
//! DHT operations are integrated into the NetworkLayer which owns the
//! Kademlia behaviour. This module provides key namespace constants
//! and helper types for DHT interactions.

/// DHT key namespace prefixes for consistent key formatting.
pub mod namespaces {
    pub const PEER: &str = "/synthread/peer/";
    pub const CHAT_INBOX: &str = "/synthread/chat/inbox/";
    pub const FRIEND: &str = "/synthread/friend/";
}

/// Build a DHT key from a namespace and identifier.
pub fn key_for_peer(namespace: &str, peer_id: &str) -> Vec<u8> {
    format!("{}{}", namespace, peer_id).into_bytes()
}
