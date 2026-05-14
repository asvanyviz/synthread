pub mod identity;
pub mod dht;
pub mod peer;
pub mod plugin;
pub mod security;
pub mod network;
pub mod api;

/// Core library API re-exports
pub use identity::IdentityManager;
pub use peer::PeerManager;
pub use plugin::{Plugin, PluginContext, PluginManager};
