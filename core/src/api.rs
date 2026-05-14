//! Localhost JSON API — axum-based HTTP server for agent/frontend communication

use axum::Router;
use tracing::info;

pub struct ApiServer {
    // axum Router + TcpListener
}

impl ApiServer {
    pub fn new() -> Self {
        info!("API server initialized (stub)");
        Self {}
    }

    /// Build the full router with all endpoints
    pub fn router(&self) -> Router {
        Router::new()
            .route("/help", axum::routing::get(help_handler))
            .route("/status", axum::routing::get(status_handler))
    }

    /// Start the API server on the given port
    pub async fn start(&self, _port: u16) -> Result<(), String> {
        info!("API server would start on port {}", _port);
        Ok(())
    }
}

// Handlers (stubs)

async fn help_handler() -> &'static str {
    "Synthread API — /help endpoint (stub)"
}

async fn status_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "peer_id": "not_generated_yet",
        "version": "0.1.0",
        "mode": "stub",
        "uptime": 0,
        "connected_peers": 0,
        "plugins": []
    }))
}
