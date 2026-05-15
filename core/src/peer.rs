//! Peer Manager — connection lifecycle, friend relationships, prioritization.
//!
//! Manages peer state (friends, known peers, connections) and provides
//! an API for friend requests, priority connections, and peer discovery.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relationship {
    Friend,
    Known,
    Discovered,
}

impl std::fmt::Display for Relationship {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Friend => write!(f, "friend"),
            Self::Known => write!(f, "known"),
            Self::Discovered => write!(f, "discovered"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Internal peer record stored in PeerManager.
#[derive(Debug, Clone)]
struct PeerRecord {
    info: PeerInfo,
}

fn now_iso() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Simple ISO-8601 without chrono dependency in tests
    chrono::DateTime::from_timestamp(dur.as_secs() as i64, 0)
        .unwrap_or_default()
        .to_rfc3339()
}

pub struct PeerManager {
    peers: HashMap<String, PeerRecord>,
    friends: HashMap<String, FriendInfo>,
    pending_requests: Vec<FriendRequest>,
    prioritized: Vec<String>,
    local_peer_id: String,
}

impl PeerManager {
    pub fn new(local_peer_id: String) -> Self {
        info!("Peer manager initialized for {}", local_peer_id);
        Self {
            peers: HashMap::new(),
            friends: HashMap::new(),
            pending_requests: Vec::new(),
            prioritized: Vec::new(),
            local_peer_id,
        }
    }

    // ── Peer discovery & tracking ──

    /// Add or update a known peer.
    pub fn upsert_peer(
        &mut self,
        peer_id: &str,
        addresses: Vec<String>,
        capabilities: Vec<String>,
    ) {
        let is_new = !self.peers.contains_key(peer_id);
        let record = self
            .peers
            .entry(peer_id.to_string())
            .or_insert_with(|| PeerRecord {
                info: PeerInfo {
                    peer_id: peer_id.to_string(),
                    addresses: Vec::new(),
                    relationship: Relationship::Discovered,
                    priority: false,
                    connection: ConnectionMode::OnDemand,
                    capabilities: Vec::new(),
                    last_seen: None,
                    latency_ms: None,
                },
            });
        record.info.addresses = addresses;
        record.info.capabilities = capabilities;
        record.info.last_seen = Some(now_iso());
        if self.friends.contains_key(peer_id) {
            record.info.relationship = Relationship::Friend;
        } else if is_new {
            record.info.relationship = Relationship::Discovered;
        } else {
            record.info.relationship = Relationship::Known;
        }
        if is_new {
            info!("New peer discovered: {}", peer_id);
        }
    }

    /// Mark a peer as connected with latency.
    pub fn set_connected(&mut self, peer_id: &str, latency_ms: u64) {
        if let Some(record) = self.peers.get_mut(peer_id) {
            record.info.latency_ms = Some(latency_ms);
            record.info.last_seen = Some(now_iso());
        }
    }

    /// Set peer latency.
    pub fn set_latency(&mut self, peer_id: &str, latency_ms: u64) {
        if let Some(record) = self.peers.get_mut(peer_id) {
            record.info.latency_ms = Some(latency_ms);
        }
    }

    /// Check if a peer is known.
    pub fn is_known(&self, peer_id: &str) -> bool {
        self.peers.contains_key(peer_id)
    }

    /// List all known peers.
    pub fn list_known(&self) -> Vec<PeerInfo> {
        self.peers.values().map(|r| r.info.clone()).collect()
    }

    /// List connected peers (those with latency set).
    pub fn list_connected(&self) -> Vec<PeerInfo> {
        self.peers
            .values()
            .filter(|r| r.info.latency_ms.is_some())
            .map(|r| r.info.clone())
            .collect()
    }

    // ── Priority connections ──

    /// Set peer priority for persistent connection and keepalive.
    pub fn set_priority(&mut self, peer_id: &str, enabled: bool, _keepalive_sec: u64) {
        if enabled {
            if !self.prioritized.contains(&peer_id.to_string()) {
                self.prioritized.push(peer_id.to_string());
            }
            if let Some(record) = self.peers.get_mut(peer_id) {
                record.info.priority = true;
                record.info.connection = ConnectionMode::Persistent;
            }
            info!("Peer {} set to priority", peer_id);
        } else {
            self.prioritized.retain(|p| p != peer_id);
            if let Some(record) = self.peers.get_mut(peer_id) {
                record.info.priority = false;
                record.info.connection = ConnectionMode::OnDemand;
            }
        }
    }

    /// Get all prioritized peers.
    pub fn get_prioritized(&self) -> Vec<PeerInfo> {
        self.prioritized
            .iter()
            .filter_map(|id| self.peers.get(id).map(|r| r.info.clone()))
            .collect()
    }

    /// Check if a peer is prioritized.
    pub fn is_prioritized(&self, peer_id: &str) -> bool {
        self.prioritized.contains(&peer_id.to_string())
    }

    // ── Friend management ──

    /// Send a friend request to a peer (local tracking).
    pub fn send_friend_request(&mut self, peer_id: &str) -> Result<(), String> {
        if self.friends.contains_key(peer_id) {
            return Err(format!("{} is already a friend", peer_id));
        }
        // In real P2P, this sends a message to the peer.
        // For now, record locally that we've initiated a request.
        info!("Friend request sent to {}", peer_id);
        Ok(())
    }

    /// Receive a friend request from a peer.
    pub fn receive_friend_request(&mut self, from: &str) {
        let request = FriendRequest {
            from: from.to_string(),
            timestamp: now_iso(),
        };
        // Deduplicate
        if !self.pending_requests.iter().any(|r| r.from == from) {
            self.pending_requests.push(request);
            info!("Friend request received from {}", from);
        }
    }

    /// Accept a pending friend request.
    pub fn friend_accept(&mut self, peer_id: &str) -> Result<(), String> {
        let had_request = self.pending_requests.iter().any(|r| r.from == peer_id);
        if !had_request && !self.peers.contains_key(peer_id) {
            return Err(format!("no pending request or known peer: {}", peer_id));
        }

        // Remove from pending
        self.pending_requests.retain(|r| r.from != peer_id);

        // Add to friends
        let friend = FriendInfo {
            peer_id: peer_id.to_string(),
            added_at: now_iso(),
            last_seen: self
                .peers
                .get(peer_id)
                .and_then(|r| r.info.last_seen.clone()),
        };
        self.friends.insert(peer_id.to_string(), friend);

        // Upgrade relationship
        if let Some(record) = self.peers.get_mut(peer_id) {
            record.info.relationship = Relationship::Friend;
        }

        info!("Friend request from {} accepted", peer_id);
        Ok(())
    }

    /// Remove a friend.
    pub fn friend_remove(&mut self, peer_id: &str) -> Result<(), String> {
        if self.friends.remove(peer_id).is_none() {
            return Err(format!("{} is not a friend", peer_id));
        }
        // Downgrade to Known
        if let Some(record) = self.peers.get_mut(peer_id) {
            record.info.relationship = Relationship::Known;
        }
        self.prioritized.retain(|p| p != peer_id);
        info!("Friend removed: {}", peer_id);
        Ok(())
    }

    /// List all friends.
    pub fn list_friends(&self) -> Vec<FriendInfo> {
        self.friends.values().cloned().collect()
    }

    /// Get pending friend requests.
    pub fn pending_friend_requests(&self) -> Vec<FriendRequest> {
        self.pending_requests.clone()
    }

    /// Check if a peer is a friend.
    pub fn is_friend(&self, peer_id: &str) -> bool {
        self.friends.contains_key(peer_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upsert_and_list() {
        let mut pm = PeerManager::new("self".into());
        pm.upsert_peer(
            "alice",
            vec!["/ip4/1.2.3.4/tcp/9000".into()],
            vec!["chat/v1".into()],
        );
        assert_eq!(pm.list_known().len(), 1);
        assert!(pm.is_known("alice"));
        assert!(!pm.is_known("bob"));
    }

    #[test]
    fn test_friend_lifecycle() {
        let mut pm = PeerManager::new("self".into());
        pm.upsert_peer("alice", vec![], vec![]);
        pm.receive_friend_request("alice");
        assert_eq!(pm.pending_friend_requests().len(), 1);

        pm.friend_accept("alice").unwrap();
        assert!(pm.is_friend("alice"));
        assert_eq!(pm.pending_friend_requests().len(), 0);

        pm.friend_remove("alice").unwrap();
        assert!(!pm.is_friend("alice"));
    }

    #[test]
    fn test_priority() {
        let mut pm = PeerManager::new("self".into());
        pm.upsert_peer("alice", vec![], vec![]);
        pm.set_priority("alice", true, 10);
        assert!(pm.is_prioritized("alice"));
        assert_eq!(pm.get_prioritized().len(), 1);

        pm.set_priority("alice", false, 0);
        assert!(!pm.is_prioritized("alice"));
    }

    #[test]
    fn test_dedup_friend_request() {
        let mut pm = PeerManager::new("self".into());
        pm.receive_friend_request("alice");
        pm.receive_friend_request("alice");
        assert_eq!(pm.pending_friend_requests().len(), 1);
    }
}
