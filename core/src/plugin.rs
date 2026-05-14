//! Plugin Manager — plugin lifecycle, isolation, and permission model

use std::collections::HashMap;
use tracing::info;

/// Trait that every plugin must implement
pub trait Plugin: Send + Sync {
    /// Unique plugin identifier (e.g., "chat-dm")
    fn id(&self) -> &str;

    /// Semantic version
    fn version(&self) -> &str;

    /// Capabilities this plugin provides (e.g., ["chat/v1"])
    fn capabilities(&self) -> Vec<String>;

    /// Called when the plugin is loaded
    fn on_load(&mut self, ctx: PluginContext);

    /// Called when the plugin is unloaded
    fn on_unload(&mut self);

    /// Handle an incoming message from a peer
    fn on_message(&mut self, peer_id: &str, protocol: &str, data: &[u8]);
}

/// Context passed to plugins — restricted access to core
pub struct PluginContext {
    pub store: PluginStore,
    pub peer_id: String,
}

impl PluginContext {
    /// Request a permission (future: full permission model)
    pub fn request_permission(&self, _perm: &str) -> bool {
        // Phase 1: basic allow-all for trusted plugins
        true
    }
}

/// Isolated key-value store per plugin
pub struct PluginStore {
    inner: HashMap<String, Vec<u8>>,
}

impl PluginStore {
    pub fn new() -> Self {
        Self { inner: HashMap::new() }
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
        Self { plugins: HashMap::new() }
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        info!("Plugin registered: {} v{}", plugin.id(), plugin.version());
        self.plugins.insert(plugin.id().to_string(), plugin);
    }

    pub fn get(&self, id: &str) -> Option<&dyn Plugin> {
        self.plugins.get(id).map(|p| p.as_ref())
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Box<dyn Plugin>> {
        self.plugins.get_mut(id)
    }

    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    pub fn load_all(&mut self, peer_id: &str) {
        for (_, plugin) in self.plugins.iter_mut() {
            let ctx = PluginContext {
                store: PluginStore::new(),
                peer_id: peer_id.to_string(),
            };
            plugin.on_load(ctx);
        }
    }

    pub fn unload_all(&mut self) {
        for (_, plugin) in self.plugins.iter_mut() {
            plugin.on_unload();
        }
    }

    pub fn route_message(&mut self, peer_id: &str, protocol: &str, data: &[u8]) {
        for (_, plugin) in self.plugins.iter_mut() {
            plugin.on_message(peer_id, protocol, data);
        }
    }
}
