//! Chat DM HTTP routes — REST API for sending/receiving encrypted messages.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ChatDmPlugin;

/// Shared state for chat endpoints.
pub struct ChatState {
    pub plugin: RwLock<ChatDmPlugin>,
}

pub type SharedChatState = Arc<ChatState>;

pub fn routes() -> Router<SharedChatState> {
    Router::new()
        .route("/api/chat/send", post(send_handler))
        .route("/api/chat/messages", get(messages_handler))
        .route("/api/chat/peers", get(peers_handler))
        .route("/api/chat/messages/{msg_id}", delete(delete_handler))
        .route("/api/chat/inbox/pending", get(pending_handler))
}

// ── Request/Response types ──

#[derive(Deserialize)]
struct SendRequest {
    to: String,
    text: String,
}

#[derive(Serialize)]
struct SendResponse {
    msg_id: String,
    status: String,
}

#[derive(Deserialize)]
struct MessagesQuery {
    peer: Option<String>,
    since: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    50
}

// ── Handlers ──

async fn send_handler(
    State(state): State<SharedChatState>,
    Json(req): Json<SendRequest>,
) -> Json<serde_json::Value> {
    let mut plugin = state.plugin.write().await;
    let placeholder_pubkey = [0u8; 32];
    let signer = |data: &[u8]| {
        vec![0u8; 64]
    };

    match plugin.send_dm(&req.to, &placeholder_pubkey, &req.text, signer) {
        Ok(msg_id) => Json(serde_json::json!({
            "msg_id": msg_id,
            "status": "sent"
        })),
        Err(e) => Json(serde_json::json!({
            "error": e
        })),
    }
}

async fn messages_handler(
    State(state): State<SharedChatState>,
    Query(query): Query<MessagesQuery>,
) -> Json<serde_json::Value> {
    let plugin = state.plugin.read().await;
    let msgs = plugin.get_messages(
        &query.peer.unwrap_or_default(),
        query.since.as_deref(),
        query.limit,
    );
    Json(serde_json::json!(msgs))
}

async fn peers_handler(
    State(_state): State<SharedChatState>,
) -> Json<serde_json::Value> {
    // Peer list comes from PeerManager via the main API
    Json(serde_json::json!([]))
}

async fn delete_handler(
    State(state): State<SharedChatState>,
    Path(msg_id): Path<String>,
) -> Json<serde_json::Value> {
    let mut plugin = state.plugin.write().await;
    // Delete requires knowing which peer the message is from;
    // iterate all peers
    let plugin_ref = &mut *plugin;
    let deleted = false; // TODO: iterate store peers
    Json(serde_json::json!({ "ok": deleted }))
}

async fn pending_handler(
    State(state): State<SharedChatState>,
) -> Json<serde_json::Value> {
    let plugin = state.plugin.read().await;
    // Return list of peers with pending (undelivered) messages
    let pending: Vec<serde_json::Value> = vec![]; // TODO: iterate store
    Json(serde_json::json!(pending))
}
