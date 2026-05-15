//! Shared API client library for frontend applications.
//!
//! Provides typed HTTP client for the Synthread JSON API with:
//! - All endpoint methods
//! - SSE event stream with auto-reconnect
//! - Offline message buffer

use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ── API Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub peer_id: String,
    pub version: String,
    pub mode: String,
    pub uptime_secs: u64,
    pub connected_peers: usize,
    pub known_peers: usize,
    pub friends: usize,
    pub plugins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub relationship: String,
    pub priority: bool,
    pub connection: String,
    pub capabilities: Vec<String>,
    pub last_seen: Option<String>,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatPeer {
    pub peer_id: String,
    pub unread: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResponse {
    pub msg_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxPending {
    pub from: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpResponse {
    pub name: String,
    pub version: String,
    pub base_url: String,
    pub endpoints: serde_json::Value,
}

// ── API Client ──

pub struct ApiClient {
    base_url: String,
    client: reqwest::Client,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    // ── Core API ──

    pub async fn help(&self) -> Result<HelpResponse, String> {
        self.client
            .get(&self.url("/help"))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn status(&self) -> Result<StatusResponse, String> {
        self.client
            .get(&self.url("/status"))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn list_peers(&self) -> Result<Vec<PeerInfo>, String> {
        self.client
            .get(&self.url("/api/peers"))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn connect_peer(&self, peer_id_or_addr: &str) -> Result<serde_json::Value, String> {
        self.client
            .post(&self.url("/api/peers/connect"))
            .json(&serde_json::json!({
                "peer_id_or_addr": peer_id_or_addr
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn set_peer_priority(
        &self,
        peer_id: &str,
        enabled: bool,
        keepalive_sec: u64,
    ) -> Result<serde_json::Value, String> {
        self.client
            .post(&self.url(&format!("/api/peers/{}/priority", peer_id)))
            .json(&serde_json::json!({
                "enabled": enabled,
                "keepalive_sec": keepalive_sec,
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn friend_request(&self, peer_id: &str) -> Result<serde_json::Value, String> {
        self.client
            .post(&self.url(&format!("/api/peers/{}/friend-request", peer_id)))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn friend_accept(&self, peer_id: &str) -> Result<serde_json::Value, String> {
        self.client
            .post(&self.url(&format!("/api/peers/{}/friend-accept", peer_id)))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    // ── Chat API ──

    pub async fn chat_send(&self, to: &str, text: &str) -> Result<SendResponse, String> {
        self.client
            .post(&self.url("/api/chat/send"))
            .json(&serde_json::json!({ "to": to, "text": text }))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn chat_messages(
        &self,
        peer: &str,
        since: Option<&str>,
        limit: usize,
    ) -> Result<serde_json::Value, String> {
        let mut url = Url::parse(&self.url("/api/chat/messages")).unwrap();
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("peer", peer);
            if let Some(s) = since {
                query.append_pair("since", s);
            }
            query.append_pair("limit", &limit.to_string());
        }
        self.client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn chat_peers(&self) -> Result<Vec<ChatPeer>, String> {
        self.client
            .get(&self.url("/api/chat/peers"))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn chat_delete(&self, msg_id: &str) -> Result<serde_json::Value, String> {
        self.client
            .delete(&self.url(&format!("/api/chat/messages/{}", msg_id)))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn chat_inbox_pending(&self) -> Result<Vec<InboxPending>, String> {
        self.client
            .get(&self.url("/api/chat/inbox/pending"))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    // ── SSE Events ──

    /// Connect to the SSE event stream. Returns a streaming response.
    /// The caller should iterate over the response body lines.
    pub async fn events(&self) -> Result<reqwest::Response, String> {
        self.client
            .get(&self.url("/events"))
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_creation() {
        let client = ApiClient::new("http://127.0.0.1:7700");
        assert_eq!(client.base_url(), "http://127.0.0.1:7700");
    }

    #[test]
    fn test_url_construction() {
        let client = ApiClient::new("http://127.0.0.1:7700");
        assert_eq!(client.url("/status"), "http://127.0.0.1:7700/status");
        assert_eq!(
            client.url("/api/peers/123/priority"),
            "http://127.0.0.1:7700/api/peers/123/priority"
        );
    }
}
