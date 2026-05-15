//! Chat DM HTTP routes — REST API for sending/receiving messages

use axum::{
    routing::{delete, get, post},
    Router,
};

pub fn routes() -> Router {
    Router::new()
        .route("/api/chat/send", post(send_handler))
        .route("/api/chat/messages", get(messages_handler))
        .route("/api/chat/peers", get(peers_handler))
        .route("/api/chat/messages/{msg_id}", delete(delete_handler))
        .route("/api/chat/inbox/pending", get(pending_handler))
}

// Handler stubs (implemented in Phase 2)

async fn send_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "msg_id": "stub",
        "status": "not_implemented"
    }))
}

async fn messages_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!([]))
}

async fn peers_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!([]))
}

async fn delete_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({"ok": true}))
}

async fn pending_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!([]))
}
