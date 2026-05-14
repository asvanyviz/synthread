//! Peer Manager — connection lifecycle, friend relationships, prioritization

use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Relationship {
    Friend,
    Known,
    Discovered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionMode {
    Persistent,
    OnDemand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub relationship: Relationship,
    pub priority: bool,
    pub connection: ConnectionMode,
    pub capabilities: Vec<String>,
    pub last_seen: Option<String>,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendInfo {
    pub peer_id: String,
    pub added_at: String,
    pub last_seen: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRequest {
    pub from: String,
    pub timestamp: String,
}

pub struct PeerManager {
    // TODO: libp2p swarm + peer store
}

impl PeerManager {
    pub fn new() -> Self {
        info!("Peer manager initialized (stub)");
        Self {}
    }

    pub fn connect(&mut self, _peer_id: &str) -> Result<(), String> {
        todo!("peer connect")
    }

    pub fn disconnect(&mut self, _peer_id: &str) {
        todo!("peer disconnect")
    }

    pub fn list_connected(&self) -> Vec<PeerInfo> {
        vec![]
    }

    pub fn list_known(&self) -> Vec<PeerInfo> {
        vec![]
    }

    // Priority connections
    pub fn set_priority(&mut self, _peer_id: &str, _enabled: bool, _keepalive_sec: u64) {
        todo!("set_priority")
    }

    pub fn get_prioritized(&self) -> Vec<PeerInfo> {
        vec![]
    }

    // Friend relationships
    pub fn friend_request(&mut self, _peer_id: &str) -> Result<(), String> {
        todo!("friend_request")
    }

    pub fn friend_accept(&mut self, _peer_id: &str) -> Result<(), String> {
        todo!("friend_accept")
    }

    pub fn friend_remove(&mut self, _peer_id: &str) -> Result<(), String> {
        todo!("friend_remove")
    }

    pub fn list_friends(&self) -> Vec<FriendInfo> {
        vec![]
    }

    pub fn pending_friend_requests(&self) -> Vec<FriendRequest> {
        vec![]
    }
}
