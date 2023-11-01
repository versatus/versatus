use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::{Address, KademliaPeerId};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct BootstrapConfig {
    pub id: KademliaPeerId,
    pub udp_gossip_addr: SocketAddr,
    pub raptorq_gossip_addr: SocketAddr,
    pub kademlia_liveness_addr: SocketAddr,
    //this is a list of addresses that will receive genesis token allocations that are not part of
    // the whitelisted node addresses
    pub additional_genesis_receivers: Option<Vec<Address>>,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        BootstrapConfig {
            id: KademliaPeerId::default(),
            raptorq_gossip_addr: addr,
            kademlia_liveness_addr: addr,
            udp_gossip_addr: addr,
            additional_genesis_receivers: None,
        }
    }
}
