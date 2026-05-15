pub mod envelope;
pub mod routes;
pub mod store;

use synthread_core::plugin::{Plugin, PluginContext};

pub struct ChatDmPlugin {
    ctx: Option<PluginContext>,
    store: crate::store::MessageStore,
}

impl ChatDmPlugin {
    pub fn new() -> Self {
        Self {
            ctx: None,
            store: crate::store::MessageStore::new(),
        }
    }
}

impl Plugin for ChatDmPlugin {
    fn id(&self) -> &str {
        "chat-dm"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["chat/v1".to_string()]
    }

    fn on_load(&mut self, ctx: PluginContext) {
        tracing::info!("Chat DM plugin loaded");
        self.ctx = Some(ctx);
    }

    fn on_unload(&mut self) {
        tracing::info!("Chat DM plugin unloaded");
    }

    fn on_message(&mut self, peer_id: &str, protocol: &str, data: &[u8]) {
        if protocol == "chat/v1" {
            tracing::debug!("Chat message from {}", peer_id);
            // Deserialize MessageEnvelope, decrypt, store, notify
        }
    }
}
