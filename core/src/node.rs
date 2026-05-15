//! Synthread Node — top-level integration of all core components.
//!
//! Owns and wires together:
//! - Identity Manager (keypair)
//! - Network Layer (libp2p Swarm)
//! - Peer Manager (peer state + friends)
//! - Plugin Manager (plugin lifecycle)
//! - API Server (JSON HTTP + SSE)
//! - Encryption Layer
//!
//! The node runs an event loop that:
//! 1. Polls the swarm for network events
//! 2. Updates the peer manager
//! 3. Broadcasts SSE events
//! 4. Routes messages to plugins

use libp2p::futures::StreamExt;
use libp2p::{identity::ed25519, Multiaddr, PeerId};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::info;

use crate::api::{ApiServer, SseEvent};
use crate::identity::IdentityManager;
use crate::network::{NetworkConfig, NetworkEvent, NetworkLayer};
use crate::peer::PeerManager;
use crate::plugin::PluginManager;

/// Top-level Synthread node.
pub struct SynthreadNode {
    pub identity: IdentityManager,
    pub network: NetworkLayer,
    pub peer_manager: Arc<RwLock<PeerManager>>,
    pub plugin_manager: Arc<RwLock<PluginManager>>,
    pub api: ApiServer,
    /// Receiver for outgoing messages from plugins.
    outgoing_rx: mpsc::UnboundedReceiver<crate::plugin::OutgoingMessage>,
}

impl SynthreadNode {
    /// Create a new Synthread node.
    ///
    /// If `identity_path` is provided and exists, loads it.
    /// Otherwise generates a new identity.
    pub fn new(
        config: NetworkConfig,
        identity_path: Option<&str>,
        passphrase: Option<&str>,
    ) -> Result<Self, String> {
        // 1. Identity
        let identity = if let (Some(path), Some(pw)) = (identity_path, passphrase) {
            if IdentityManager::exists(path) {
                IdentityManager::load(path, pw).map_err(|e| format!("load identity: {}", e))?
            } else {
                let id = IdentityManager::generate();
                id.export(path, pw)
                    .map_err(|e| format!("export identity: {}", e))?;
                info!("New identity saved to {}", path);
                id
            }
        } else {
            info!("No identity path provided — generating ephemeral identity");
            IdentityManager::generate()
        };

        // 2. Convert ed25519 key to libp2p keypair
        let keypair_bytes = identity.to_keypair_bytes();
        let mut keypair_clone = keypair_bytes;
        let libp2p_keypair = ed25519::Keypair::from(
            ed25519::SecretKey::try_from_bytes(&mut keypair_clone[..32])
                .map_err(|e| format!("invalid secret key: {}", e))?,
        );

        let peer_id = identity.peer_id().to_base58();

        // 3. Peer Manager
        let peer_manager = Arc::new(RwLock::new(PeerManager::new(peer_id.clone())));

        // 4. Plugin Manager
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
        let mut plugin_manager = PluginManager::new();
        plugin_manager.load_all(&peer_id, outgoing_tx, Some(identity.to_keypair_bytes()));
        let plugin_manager = Arc::new(RwLock::new(plugin_manager));

        // 5. Network Layer
        let network = NetworkLayer::new(&libp2p_keypair, config.bootstrap_peers.clone())?;

        // 6. API Server
        let api = ApiServer::new(
            Arc::clone(&peer_manager),
            Arc::clone(&plugin_manager),
            peer_id.clone(),
        );

        info!("Synthread node initialized — peer ID: {}", peer_id);

        Ok(Self {
            identity,
            network,
            peer_manager,
            plugin_manager,
            api,
            outgoing_rx,
        })
    }

    /// Start listening and bootstrap the DHT.
    pub fn start_listening(&mut self, addrs: &[Multiaddr]) -> Result<(), String> {
        self.network.start(addrs)
    }

    /// Dial a peer by address or PeerId.
    pub fn dial(&mut self, addr: Multiaddr) -> Result<(), String> {
        self.network.dial(addr)
    }

    /// Run the main event loop (blocking).
    ///
    /// This should be spawned as a background task.
    pub async fn run_event_loop(&mut self) {
        info!("Event loop started");
        loop {
            tokio::select! {
                // Network events
                net_event = self.network.next_event() => {
                    if let Some(event) = net_event {
                        self.process_network_event(event).await;
                    }
                }
                // Outgoing messages from plugins
                outgoing = self.outgoing_rx.recv() => {
                    if let Some(msg) = outgoing {
                        self.dispatch_outgoing(msg).await;
                    } else {
                        // Channel closed — all plugins dropped
                        info!("Outgoing message channel closed");
                        break;
                    }
                }
            }
        }
    }

    /// Dispatch an outgoing message from a plugin to the network.
    async fn dispatch_outgoing(&mut self, msg: crate::plugin::OutgoingMessage) {
        let peer_id: PeerId = match msg.target_peer.parse() {
            Ok(pid) => pid,
            Err(_) => {
                tracing::warn!("Invalid peer ID in outgoing message: {}", msg.target_peer);
                return;
            }
        };

        // Store in the DHT as an inbox entry for offline delivery
        use libp2p::kad::Record;
        let inbox_key =
            crate::dht::key_for_peer(crate::dht::namespaces::CHAT_INBOX, &msg.target_peer);
        let record = Record::new(inbox_key, msg.data);
        let _ = self
            .network
            .swarm
            .behaviour_mut()
            .kademlia
            .put_record(record, libp2p::kad::Quorum::One);

        tracing::debug!(
            "Dispatched outgoing {} message to {}",
            msg.protocol,
            msg.target_peer
        );
    }

    /// Process a network event and update local state.
    async fn process_network_event(&mut self, event: NetworkEvent) {
        match event {
            NetworkEvent::PeerConnected { peer_id } => {
                let id_str = peer_id.to_base58();
                {
                    let mut peers = self.peer_manager.write().await;
                    peers.set_connected(&id_str, 0);
                }
                self.api
                    .state()
                    .broadcast_event(SseEvent::PeerConnected { peer_id: id_str });
            }
            NetworkEvent::PeerDisconnected { peer_id } => {
                let id_str = peer_id.to_base58();
                self.api
                    .state()
                    .broadcast_event(SseEvent::PeerDisconnected { peer_id: id_str });
            }
            NetworkEvent::PeerIdentified {
                peer_id,
                agent_version,
                listen_addrs,
            } => {
                let id_str = peer_id.to_base58();
                let addrs: Vec<String> = listen_addrs.iter().map(|a| a.to_string()).collect();
                let caps = if agent_version.contains("chat") {
                    vec!["chat/v1".to_string()]
                } else {
                    vec![]
                };
                {
                    let mut peers = self.peer_manager.write().await;
                    peers.upsert_peer(&id_str, addrs, caps);
                }
                // Add addresses to Kademlia
                for addr in &listen_addrs {
                    self.network
                        .swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr.clone());
                }
            }
            NetworkEvent::NewListenAddr { address } => {
                info!("Listening on {}", address);
            }
            NetworkEvent::PublicAddressConfirmed { address } => {
                info!("Public address confirmed: {}", address);
            }
            NetworkEvent::FriendRequestReceived { peer_id, request } => {
                let id_str = peer_id.to_base58();
                info!("Friend request from {}: {:?}", id_str, request);
                {
                    let mut peers = self.peer_manager.write().await;
                    peers.receive_friend_request(&id_str);
                }
                self.api
                    .state()
                    .broadcast_event(SseEvent::FriendRequest { from: id_str });
            }
            NetworkEvent::FriendResponseReceived { peer_id, response } => {
                let id_str = peer_id.to_base58();
                info!(
                    "Friend response from {}: accepted={}",
                    id_str, response.accepted
                );
                if response.accepted {
                    let mut peers = self.peer_manager.write().await;
                    let _ = peers.friend_accept(&id_str);
                    self.api
                        .state()
                        .broadcast_event(SseEvent::FriendAccepted { peer_id: id_str });
                }
            }
        }
    }
}

/// Convenience: create a node from parsed config.
pub struct NodeBuilder {
    pub config: NetworkConfig,
    pub identity_path: Option<String>,
    pub passphrase: Option<String>,
}

impl NodeBuilder {
    pub fn new(config: NetworkConfig) -> Self {
        Self {
            config,
            identity_path: None,
            passphrase: None,
        }
    }

    pub fn with_identity(mut self, path: &str, passphrase: &str) -> Self {
        self.identity_path = Some(path.to_string());
        self.passphrase = Some(passphrase.to_string());
        self
    }

    pub fn build(self) -> Result<SynthreadNode, String> {
        SynthreadNode::new(
            self.config,
            self.identity_path.as_deref(),
            self.passphrase.as_deref(),
        )
    }
}
