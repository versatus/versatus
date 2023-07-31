use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::KademliaPeerId;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct BootstrapConfig {
    pub id: KademliaPeerId,
    pub udp_gossip_addr: SocketAddr,
    pub raptorq_gossip_addr: SocketAddr,
    pub kademlia_liveness_addr: SocketAddr,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        BootstrapConfig {
            id: KademliaPeerId::default(),
            raptorq_gossip_addr: addr,
            kademlia_liveness_addr: addr,
            udp_gossip_addr: addr,
        }
    }
}
