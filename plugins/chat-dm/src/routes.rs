//! Chat DM HTTP routes — REST API for sending/receiving encrypted messages.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
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
    match plugin.send_dm(&req.to, &req.text) {
        Ok(msg_id) => Json(serde_json::json!({
            "msg_id": msg_id,
            "status": "sent"
        })),
        Err(e) => Json(serde_json::json!({ "error": e })),
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
    State(state): State<SharedChatState>,
) -> Json<serde_json::Value> {
    let plugin = state.plugin.read().await;
    let peers: Vec<serde_json::Value> = plugin
        .pending_peers()
        .iter()
        .map(|(id, count)| {
            serde_json::json!({
                "peer_id": id,
                "unread": count,
            })
        })
        .collect();
    Json(serde_json::json!(peers))
}

async fn delete_handler(
    State(state): State<SharedChatState>,
    Path(msg_id): Path<String>,
) -> Json<serde_json::Value> {
    let mut plugin = state.plugin.write().await;
    let deleted = plugin.delete_message(&msg_id);
    Json(serde_json::json!({ "ok": deleted }))
}

async fn pending_handler(
    State(state): State<SharedChatState>,
) -> Json<serde_json::Value> {
    let plugin = state.plugin.read().await;
    let pending: Vec<serde_json::Value> = plugin
        .pending_peers()
        .iter()
        .map(|(id, count)| {
            serde_json::json!({
                "from": id,
                "count": count,
            })
        })
        .collect();
    Json(serde_json::json!(pending))
}
