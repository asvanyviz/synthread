//! Network Layer — libp2p transport setup (TCP, Noise, Yamux)

use tracing::info;

pub struct NetworkLayer {
    // libp2p Swarm will live here
}

impl NetworkLayer {
    pub fn new() -> Self {
        info!("Network layer initialized (stub)");
        Self {}
    }

    /// Start listening on configured addresses
    pub async fn start(&mut self, _listen_addrs: &[String]) -> Result<(), String> {
        todo!("network start")
    }

    /// Stop the network layer
    pub async fn stop(&mut self) {
        todo!("network stop")
    }

    /// Get local listening addresses
    pub fn listen_addrs(&self) -> Vec<String> {
        vec![]
    }

    /// Get external (public) addresses discovered via AutoNAT
    pub fn external_addrs(&self) -> Vec<String> {
        vec![]
    }
}
