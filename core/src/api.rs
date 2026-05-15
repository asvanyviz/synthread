//! Localhost JSON API — axum-based HTTP server for agent/frontend communication.
//!
//! Base URL: `http://127.0.0.1:7700`
//!
//! Endpoints:
//!   GET  /help       → full API description
//!   GET  /status     → peer ID, version, uptime, connected peers, plugins
//!   GET  /api/peers  → list known peers
//!   POST /api/peers/connect         { peer_id_or_addr }
//!   POST /api/peers/<id>/priority    { enabled, keepalive_sec }
//!   POST /api/peers/<id>/friend-request
//!   POST /api/peers/<id>/friend-accept
//!   GET  /events     → SSE event stream
//!   GET  /dashboard  → WebUI status page

use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::peer::PeerManager;
use crate::plugin::PluginManager;

/// Shared application state for the API server.
pub struct AppState {
    pub peer_manager: Arc<RwLock<PeerManager>>,
    pub plugin_manager: Arc<RwLock<PluginManager>>,
    pub local_peer_id: String,
    pub version: String,
    pub start_time: std::time::Instant,
    /// Broadcast channel for SSE events.
    pub event_tx: broadcast::Sender<SseEvent>,
}

impl AppState {
    fn new(
        peer_manager: Arc<RwLock<PeerManager>>,
        plugin_manager: Arc<RwLock<PluginManager>>,
        local_peer_id: String,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            peer_manager,
            plugin_manager,
            local_peer_id,
            version: env!("CARGO_PKG_VERSION").to_string(),
            start_time: std::time::Instant::now(),
            event_tx,
        }
    }

    /// Broadcast an SSE event to all listeners.
    pub fn broadcast_event(&self, event: SseEvent) {
        let _ = self.event_tx.send(event);
    }
}

/// SSE event types for the /events stream.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "event", content = "data")]
pub enum SseEvent {
    #[serde(rename = "message")]
    Message {
        from: String,
        msg_id: String,
        preview: String,
    },
    #[serde(rename = "friend_request")]
    FriendRequest { from: String },
    #[serde(rename = "friend_accepted")]
    FriendAccepted { peer_id: String },
    #[serde(rename = "peer_connected")]
    PeerConnected { peer_id: String },
    #[serde(rename = "peer_disconnected")]
    PeerDisconnected { peer_id: String },
}

impl SseEvent {
    fn to_sse(&self) -> (String, String) {
        match self {
            Self::Message {
                from,
                msg_id,
                preview,
            } => (
                "message".to_string(),
                serde_json::json!({ "from": from, "msg_id": msg_id, "preview": preview })
                    .to_string(),
            ),
            Self::FriendRequest { from } => (
                "friend_request".to_string(),
                serde_json::json!({ "from": from }).to_string(),
            ),
            Self::FriendAccepted { peer_id } => (
                "friend_accepted".to_string(),
                serde_json::json!({ "peer_id": peer_id }).to_string(),
            ),
            Self::PeerConnected { peer_id } => (
                "peer_connected".to_string(),
                serde_json::json!({ "peer_id": peer_id }).to_string(),
            ),
            Self::PeerDisconnected { peer_id } => (
                "peer_disconnected".to_string(),
                serde_json::json!({ "peer_id": peer_id }).to_string(),
            ),
        }
    }
}

type SharedState = Arc<AppState>;

pub struct ApiServer {
    state: SharedState,
}

impl ApiServer {
    pub fn new(
        peer_manager: Arc<RwLock<PeerManager>>,
        plugin_manager: Arc<RwLock<PluginManager>>,
        local_peer_id: String,
    ) -> Self {
        info!("API server initialized");
        Self {
            state: Arc::new(AppState::new(peer_manager, plugin_manager, local_peer_id)),
        }
    }

    /// Return a reference to the shared state (for external use).
    pub fn state(&self) -> SharedState {
        self.state.clone()
    }

    /// Build the full router with all endpoints.
    pub fn router(&self) -> Router {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        Router::new()
            .route("/help", get(help_handler))
            .route("/status", get(status_handler))
            .route("/api/peers", get(list_peers_handler))
            .route("/api/peers/connect", post(connect_peer_handler))
            .route("/api/peers/{id}/priority", post(set_priority_handler))
            .route(
                "/api/peers/{id}/friend-request",
                post(friend_request_handler),
            )
            .route("/api/peers/{id}/friend-accept", post(friend_accept_handler))
            .route("/events", get(sse_handler))
            .route("/dashboard", get(dashboard_handler))
            .layer(cors)
            .with_state(self.state.clone())
    }

    /// Start the API server on the given port.
    pub async fn start(&self, port: u16) -> Result<(), String> {
        let addr = format!("127.0.0.1:{}", port);
        info!("API server starting on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| format!("bind {}: {}", addr, e))?;

        axum::serve(listener, self.router())
            .await
            .map_err(|e| format!("server error: {}", e))?;

        Ok(())
    }
}

// ── Handlers ──

async fn help_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "Synthread API",
        "version": "1.0",
        "base_url": "http://127.0.0.1:7700",
        "endpoints": {
            "GET /help": "This document",
            "GET /status": "Server status (peer_id, version, uptime, connected_peers, plugins)",
            "GET /api/peers": "List known peers",
            "POST /api/peers/connect": "Connect to a peer (body: { peer_id_or_addr })",
            "POST /api/peers/{id}/priority": "Set peer priority (body: { enabled, keepalive_sec })",
            "POST /api/peers/{id}/friend-request": "Send friend request",
            "POST /api/peers/{id}/friend-accept": "Accept friend request",
            "GET /events": "SSE event stream"
        }
    }))
}

async fn status_handler(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let peers = state.peer_manager.read().await;
    let plugins = state.plugin_manager.read().await;
    let uptime = state.start_time.elapsed().as_secs();

    Json(serde_json::json!({
        "peer_id": state.local_peer_id,
        "version": state.version,
        "mode": "headless",
        "uptime_secs": uptime,
        "connected_peers": peers.list_connected().len(),
        "known_peers": peers.list_known().len(),
        "friends": peers.list_friends().len(),
        "plugins": plugins.list_plugins(),
    }))
}

async fn list_peers_handler(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let peers = state.peer_manager.read().await;
    Json(serde_json::json!(peers.list_known()))
}

#[derive(serde::Deserialize)]
struct ConnectPeerBody {
    peer_id_or_addr: String,
}

async fn connect_peer_handler(
    State(state): State<SharedState>,
    Json(body): Json<ConnectPeerBody>,
) -> Json<serde_json::Value> {
    // Register as known peer if not already
    {
        let mut peers = state.peer_manager.write().await;
        peers.upsert_peer(&body.peer_id_or_addr, vec![], vec![]);
    }

    // TODO: actually dial via NetworkLayer (needs cross-component wiring)
    Json(serde_json::json!({
        "status": "registered",
        "peer_id_or_addr": body.peer_id_or_addr,
        "note": "dial requires NetworkLayer integration (Phase 1 WIP)"
    }))
}

#[derive(serde::Deserialize)]
struct PriorityBody {
    enabled: bool,
    #[serde(default = "default_keepalive")]
    keepalive_sec: u64,
}

fn default_keepalive() -> u64 {
    10
}

async fn set_priority_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<PriorityBody>,
) -> Json<serde_json::Value> {
    let mut peers = state.peer_manager.write().await;
    peers.set_priority(&id, body.enabled, body.keepalive_sec);
    Json(serde_json::json!({ "ok": true }))
}

async fn friend_request_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let mut peers = state.peer_manager.write().await;
    match peers.send_friend_request(&id) {
        Ok(()) => Json(serde_json::json!({ "ok": true })),
        Err(e) => Json(serde_json::json!({ "error": e })),
    }
}

async fn friend_accept_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let mut peers = state.peer_manager.write().await;
    match peers.friend_accept(&id) {
        Ok(()) => {
            state.broadcast_event(SseEvent::FriendAccepted {
                peer_id: id.clone(),
            });
            Json(serde_json::json!({ "ok": true }))
        }
        Err(e) => Json(serde_json::json!({ "error": e })),
    }
}

/// SSE event stream endpoint.
async fn sse_handler(
    State(state): State<SharedState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut rx = state.event_tx.subscribe();

    let stream = async_stream::stream! {
        // Send initial connected event
        yield Ok(Event::default().event("connected").data(
            serde_json::json!({ "peer_id": state.local_peer_id }).to_string(),
        ));

        loop {
            match rx.recv().await {
                Ok(event) => {
                    let (name, data) = event.to_sse();
                    yield Ok(Event::default().event(name).data(data));
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("SSE client lagged by {} messages", n);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Embedded WebUI dashboard — minimal HTML status page.
async fn dashboard_handler(State(state): State<SharedState>) -> axum::response::Html<String> {
    let peers = state.peer_manager.read().await;
    let plugins = state.plugin_manager.read().await;
    let uptime = state.start_time.elapsed().as_secs();

    let peer_html = peers
        .list_known()
        .iter()
        .map(|p| {
            let friend_class = if p.relationship == crate::peer::Relationship::Friend {
                " friend"
            } else {
                ""
            };
            let prio = if p.priority { " ⭐" } else { "" };
            let latency = if let Some(ms) = p.latency_ms {
                format!(" {}ms", ms)
            } else {
                String::new()
            };
            format!(
                r#"<div class="peer{}">{} — {} {}{}</div>"#,
                friend_class,
                &p.peer_id[..p.peer_id.len().min(16)],
                p.relationship,
                prio,
                latency,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Synthread Dashboard</title>
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;background:#1a1a2e;color:#e0e0e0;padding:2rem}}
.card{{background:#16213e;border-radius:8px;padding:1.5rem;margin-bottom:1rem}}
h1{{color:#00d4ff;margin-bottom:1rem}}
h2{{color:#7b68ee;margin-bottom:.5rem}}
.stat{{display:flex;justify-content:space-between;padding:.5rem 0;border-bottom:1px solid #2a2a4a}}
.stat-label{{color:#888}}.stat-value{{color:#00d4ff;font-weight:bold}}
.peer{{padding:.5rem;margin:.25rem 0;background:#0f3460;border-radius:4px}}
.friend{{border-left:3px solid #00d4ff}}
#events{{max-height:300px;overflow-y:auto;font-family:monospace;font-size:.85rem}}
.event{{padding:.25rem 0}}
</style>
</head>
<body>
<h1>Synthread Dashboard</h1>
<div class="card">
<h2>Status</h2>
<div class="stat"><span class="stat-label">Peer ID</span><span class="stat-value">{peer_id}</span></div>
<div class="stat"><span class="stat-label">Version</span><span class="stat-value">{version}</span></div>
<div class="stat"><span class="stat-label">Uptime</span><span class="stat-value">{uptime}s</span></div>
<div class="stat"><span class="stat-label">Connected</span><span class="stat-value">{connected}</span></div>
<div class="stat"><span class="stat-label">Known Peers</span><span class="stat-value">{known}</span></div>
<div class="stat"><span class="stat-label">Friends</span><span class="stat-value">{friends}</span></div>
<div class="stat"><span class="stat-label">Plugins</span><span class="stat-value">{plugin_list}</span></div>
</div>
<div class="card">
<h2>Peers</h2>
{peer_html}
</div>
<div class="card">
<h2>Events</h2>
<div id="events">Connecting...</div>
</div>
<script>
const es=new EventSource('/events');
const el=document.getElementById('events');
es.onmessage=(e)=>{{const d=document.createElement('div');d.className='event';d.textContent='['+new Date().toLocaleTimeString()+'] '+e.type+': '+e.data;el.prepend(d)}};
es.addEventListener('peer_connected',(e)=>{{const d=JSON.parse(e.data);const x=document.createElement('div');x.className='event';x.innerHTML='<span style=color:#0f0>connected: '+d.peer_id+'</span>';el.prepend(x)}});
es.onerror=()=>el.textContent='Disconnected. Retrying...';
</script>
</body>
</html>"#,
        peer_id = state.local_peer_id,
        version = state.version,
        uptime = uptime,
        connected = peers.list_connected().len(),
        known = peers.list_known().len(),
        friends = peers.list_friends().len(),
        plugin_list = plugins.list_plugins().join(", "),
        peer_html = peer_html,
    );

    axum::response::Html(html)
}
