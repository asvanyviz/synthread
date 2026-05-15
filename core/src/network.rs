//! Network Layer — libp2p Swarm setup and management.
//!
//! Sets up a libp2p Swarm with:
//! - TCP transport
//! - Noise XX handshake for authenticated encryption
//! - Yamux multiplexing
//! - Kademlia DHT
//! - Identify protocol
//! - Ping (keepalive)
//! - AutoNAT for NAT detection
//! - Relay for NAT traversal fallback

use libp2p::futures::StreamExt;
use libp2p::{
    autonat, identify,
    identity::ed25519,
    kad::{self, store::MemoryStore, Mode},
    noise, ping,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, SwarmBuilder,
};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Combined network behaviour for libp2p.
#[derive(NetworkBehaviour)]
pub struct SynthreadBehaviour {
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub autonat: autonat::Behaviour,
}

/// Core network layer managing the libp2p Swarm.
pub struct NetworkLayer {
    pub swarm: libp2p::Swarm<SynthreadBehaviour>,
    pub local_peer_id: PeerId,
    bootstrap_peers: Vec<Multiaddr>,
    listen_addrs: Vec<Multiaddr>,
}

impl NetworkLayer {
    /// Create a new NetworkLayer from an ed25519 keypair.
    pub fn new(
        keypair: &ed25519::Keypair,
        bootstrap_peers: Vec<Multiaddr>,
    ) -> Result<Self, String> {
        let pk = libp2p::identity::PublicKey::from(keypair.public());
        let local_peer_id = PeerId::from(pk);

        // Build the swarm
        let swarm =
            SwarmBuilder::with_existing_identity(libp2p::identity::Keypair::from(keypair.clone()))
                .with_tokio()
                .with_tcp(
                    tcp::Config::default().nodelay(true),
                    noise::Config::new,
                    yamux::Config::default,
                )
                .map_err(|e| format!("transport build failed: {}", e))?
                .with_behaviour(|key| {
                    // Kademlia DHT
                    let mut kademlia =
                        kad::Behaviour::new(local_peer_id, MemoryStore::new(local_peer_id));
                    kademlia.set_mode(Some(Mode::Server));

                    // Identify protocol
                    let identify = identify::Behaviour::new(identify::Config::new(
                        "/synthread/1.0.0".to_string(),
                        key.public(),
                    ));

                    // Ping for keepalive
                    let ping = ping::Behaviour::new(ping::Config::new());

                    // AutoNAT for NAT detection
                    let autonat =
                        autonat::Behaviour::new(local_peer_id, autonat::Config::default());

                    Ok(SynthreadBehaviour {
                        kademlia,
                        identify,
                        ping,
                        autonat,
                    })
                })
                .map_err(|e| format!("behaviour build failed: {}", e))?
                .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60)))
                .build();

        info!("Network layer initialized — peer ID: {}", local_peer_id);

        Ok(Self {
            swarm,
            local_peer_id,
            bootstrap_peers,
            listen_addrs: Vec::new(),
        })
    }

    /// Start listening on the configured addresses and bootstrap DHT.
    pub fn start(&mut self, addrs: &[Multiaddr]) -> Result<(), String> {
        for addr in addrs {
            self.swarm
                .listen_on(addr.clone())
                .map_err(|e| format!("listen on {}: {}", addr, e))?;
            self.listen_addrs.push(addr.clone());
            info!("Listening on {}", addr);
        }

        // Bootstrap Kademlia
        for addr in &self.bootstrap_peers {
            self.swarm
                .dial(addr.clone())
                .map_err(|e| warn!("Bootstrap dial {} failed: {}", addr, e))
                .ok();
        }

        // After connecting to bootstrap peers, query closest peers to our own ID
        // to populate the routing table. This is deferred until the first peer connects.
        // (See handle_event for the actual bootstrap query.)

        Ok(())
    }

    /// Stop the network layer.
    pub fn stop(&mut self) -> Result<(), String> {
        info!("Stopping network layer...");
        // No explicit stop needed; dropping the swarm does it.
        Ok(())
    }

    /// Get local listening addresses (including discovered external)
    pub fn listen_addrs(&self) -> Vec<Multiaddr> {
        self.swarm.listeners().cloned().collect()
    }

    /// Get external (public) addresses discovered via AutoNAT or Identify.
    pub fn external_addrs(&self) -> Vec<Multiaddr> {
        self.swarm.external_addresses().cloned().collect()
    }

    /// Get the local peer ID.
    pub fn local_peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }

    /// Dial a peer by address or peer ID.
    pub fn dial(&mut self, addr: Multiaddr) -> Result<(), String> {
        self.swarm
            .dial(addr)
            .map_err(|e| format!("dial failed: {}", e))
    }

    /// Disconnect from a peer.
    pub fn disconnect(&mut self, peer_id: PeerId) -> Result<(), String> {
        self.swarm
            .disconnect_peer_id(peer_id)
            .map_err(|_| "no connection".to_string())
    }

    /// Check if connected to a peer.
    pub fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.swarm.is_connected(peer_id)
    }

    /// Get connected peers.
    pub fn connected_peers(&self) -> usize {
        self.swarm.connected_peers().count()
    }

    /// Handle a network event. Returns optional structured event.
    pub fn handle_event(
        &mut self,
        event: SwarmEvent<SynthreadBehaviourEvent>,
    ) -> Option<NetworkEvent> {
        match event {
            SwarmEvent::Behaviour(SynthreadBehaviourEvent::Kademlia(
                kad::Event::RoutingUpdated { peer, .. },
            )) => {
                debug!("Kademlia routing updated: peer {}", peer);
                None
            }
            SwarmEvent::Behaviour(SynthreadBehaviourEvent::Kademlia(
                kad::Event::UnroutablePeer { peer },
            )) => {
                debug!("Kademlia unroutable peer: {}", peer);
                None
            }
            SwarmEvent::Behaviour(SynthreadBehaviourEvent::Kademlia(
                kad::Event::OutboundQueryProgressed { id, result, .. },
            )) => {
                match result {
                    kad::QueryResult::Bootstrap(Ok(kad::BootstrapOk { peer, .. })) => {
                        info!("Kademlia bootstrap successful via {}", peer);
                    }
                    kad::QueryResult::Bootstrap(Err(e)) => {
                        warn!("Kademlia bootstrap error: {:?}", e);
                    }
                    kad::QueryResult::GetClosestPeers(Ok(kad::GetClosestPeersOk {
                        key,
                        peers,
                    })) => {
                        debug!(
                            "Kademlia query for {:?} returned {} peers",
                            std::str::from_utf8(&key).unwrap_or("<binary>"),
                            peers.len()
                        );
                    }
                    _ => {
                        debug!("Kademlia query {:?} progressed", id);
                    }
                }
                None
            }
            SwarmEvent::Behaviour(SynthreadBehaviourEvent::Identify(
                identify::Event::Received { peer_id, info, .. },
            )) => {
                info!(
                    "Identified peer {} — v{} [{}] ({})",
                    peer_id,
                    info.protocol_version,
                    info.agent_version,
                    info.listen_addrs
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );

                // Add discovered addresses to Kademlia
                for addr in &info.listen_addrs {
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr.clone());
                }

                Some(NetworkEvent::PeerIdentified {
                    peer_id,
                    agent_version: info.agent_version,
                    listen_addrs: info.listen_addrs,
                })
            }
            SwarmEvent::Behaviour(SynthreadBehaviourEvent::Ping(ping_event)) => {
                match ping_event.result {
                    Ok(rtt) => {
                        debug!("Ping {} — RTT: {}ms", ping_event.peer, rtt.as_millis());
                    }
                    Err(e) => {
                        warn!("Ping failed to {}: {}", ping_event.peer, e);
                    }
                }
                None
            }
            SwarmEvent::Behaviour(SynthreadBehaviourEvent::Autonat(
                autonat::Event::OutboundProbe(autonat::OutboundProbeEvent::Response {
                    peer,
                    address,
                    ..
                }),
            )) => {
                info!("AutoNAT: confirmed public address {} via {}", address, peer);
                Some(NetworkEvent::PublicAddressConfirmed { address })
            }
            SwarmEvent::Behaviour(SynthreadBehaviourEvent::Autonat(
                autonat::Event::StatusChanged { old, new },
            )) => {
                info!("AutoNAT status changed: {:?} → {:?}", old, new);
                None
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("New listen address: {}", address);
                Some(NetworkEvent::NewListenAddr { address })
            }
            SwarmEvent::ConnectionEstablished {
                peer_id,
                num_established,
                ..
            } => {
                info!(
                    "Connection established with {} (total: {})",
                    peer_id, num_established
                );

                // Bootstrap Kademlia when we get first connections
                if num_established.get() <= 3 {
                    self.bootstrap_kademlia();
                }

                Some(NetworkEvent::PeerConnected { peer_id })
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                num_established,
                cause,
                ..
            } => {
                info!(
                    "Connection closed: {} (remaining: {}, cause: {:?})",
                    peer_id, num_established, cause
                );
                Some(NetworkEvent::PeerDisconnected { peer_id })
            }
            SwarmEvent::Dialing { peer_id, .. } => {
                debug!("Dialing {}", peer_id.unwrap_or(PeerId::random()));
                None
            }
            SwarmEvent::IncomingConnection { .. } => {
                debug!("Incoming connection");
                None
            }
            _ => {
                debug!("Unhandled swarm event");
                None
            }
        }
    }

    /// Start a Kademlia bootstrap query.
    fn bootstrap_kademlia(&mut self) {
        let bootstrap_result = self.swarm.behaviour_mut().kademlia.bootstrap();
        match bootstrap_result {
            Ok(id) => {
                info!("Kademlia bootstrap started (query id: {:?})", id);
            }
            Err(e) => {
                warn!("Kademlia bootstrap failed: {:?}", e);
            }
        }
    }

    /// Run the event loop for a single event (async).
    /// Returns None when the swarm has no more events (should not happen in normal operation).
    pub async fn next_event(&mut self) -> Option<NetworkEvent> {
        loop {
            let event = self.swarm.next().await?;
            if let Some(net_event) = self.handle_event(event) {
                return Some(net_event);
            }
        }
    }
}

/// High-level network events for consumers.
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    PeerConnected {
        peer_id: PeerId,
    },
    PeerDisconnected {
        peer_id: PeerId,
    },
    PeerIdentified {
        peer_id: PeerId,
        agent_version: String,
        listen_addrs: Vec<Multiaddr>,
    },
    NewListenAddr {
        address: Multiaddr,
    },
    PublicAddressConfirmed {
        address: Multiaddr,
    },
}

/// Network configuration passed to NetworkLayer.
pub struct NetworkConfig {
    pub listen_addresses: Vec<Multiaddr>,
    pub bootstrap_peers: Vec<Multiaddr>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec!["/ip4/0.0.0.0/tcp/9000".parse().unwrap()],
            bootstrap_peers: vec![],
        }
    }
}
