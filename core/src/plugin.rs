//! Plugin Manager — plugin lifecycle, isolation, and permission model

use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::info;

// ── Outgoing message from plugin to core ──

/// An outgoing message from a plugin, to be dispatched by the core.
#[derive(Debug, Clone)]
pub struct OutgoingMessage {
    pub target_peer: String,
    pub protocol: String,
    pub data: Vec<u8>,
    /// If true, queue for offline delivery if peer is not connected.
    pub allow_offline: bool,
}

/// Trait that every plugin must implement.
pub trait Plugin: Send + Sync {
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn capabilities(&self) -> Vec<String>;
    fn on_load(&mut self, ctx: PluginContext);
    fn on_unload(&mut self);
    fn on_message(&mut self, peer_id: &str, protocol: &str, data: &[u8]);
}

/// Context passed to plugins — restricted access to core services.
pub struct PluginContext {
    pub store: PluginStore,
    pub peer_id: String,
    /// Channel to send outgoing messages to the core for dispatch.
    pub outgoing_tx: Option<mpsc::UnboundedSender<OutgoingMessage>>,
}

impl PluginContext {
    /// Send a message to a peer through the core network layer.
    pub fn send_message(&self, peer_id: &str, protocol: &str, data: Vec<u8>) -> Result<(), String> {
        match &self.outgoing_tx {
            Some(tx) => tx
                .send(OutgoingMessage {
                    target_peer: peer_id.to_string(),
                    protocol: protocol.to_string(),
                    data,
                    allow_offline: true,
                })
                .map_err(|e| format!("send failed: {}", e)),
            None => Err("no outgoing channel configured".to_string()),
        }
    }

    /// Request a permission.
    pub fn request_permission(&self, _perm: &str) -> bool {
        true // Phase 1: permissive
    }
}

/// Isolated key-value store per plugin.
pub struct PluginStore {
    inner: HashMap<String, Vec<u8>>,
}

impl PluginStore {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
    pub fn get(&self, key: &str) -> Option<&Vec<u8>> {
        self.inner.get(key)
    }
    pub fn set(&mut self, key: &str, value: Vec<u8>) {
        self.inner.insert(key.to_string(), value);
    }
    pub fn delete(&mut self, key: &str) {
        self.inner.remove(key);
    }
}

pub struct PluginManager {
    plugins: HashMap<String, Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        info!("Plugin manager initialized");
        Self {
            plugins: HashMap::new(),
        }
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        info!("Plugin registered: {} v{}", plugin.id(), plugin.version());
        self.plugins.insert(plugin.id().to_string(), plugin);
    }

    pub fn get(&self, id: &str) -> Option<&dyn Plugin> {
        self.plugins.get(id).map(|p| p.as_ref())
    }

    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Load all plugins with an outgoing message channel.
    pub fn load_all(&mut self, peer_id: &str, outgoing_tx: mpsc::UnboundedSender<OutgoingMessage>) {
        for (_, plugin) in self.plugins.iter_mut() {
            let ctx = PluginContext {
                store: PluginStore::new(),
                peer_id: peer_id.to_string(),
                outgoing_tx: Some(outgoing_tx.clone()),
            };
            plugin.on_load(ctx);
        }
    }

    pub fn unload_all(&mut self) {
        for (_, plugin) in self.plugins.iter_mut() {
            plugin.on_unload();
        }
    }

    /// Route an incoming message to all plugins.
    pub fn route_message(&mut self, peer_id: &str, protocol: &str, data: &[u8]) {
        for (_, plugin) in self.plugins.iter_mut() {
            plugin.on_message(peer_id, protocol, data);
        }
    }
}
