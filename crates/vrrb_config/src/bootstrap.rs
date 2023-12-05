use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::{Address, KademliaPeerId};
use serde::{Deserialize, Serialize};

use crate::BootstrapQuorumConfig;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapConfig {
    /// This is a list of addresses that will receive genesis token allocations that are not part of
    /// the whitelisted node addresses
    pub additional_genesis_receivers: Option<Vec<Address>>,
    /// Optional Genesis Quorum configuration used to bootstrap a new quorum
    pub bootstrap_quorum_config: BootstrapQuorumConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapPeerData {
    pub id: KademliaPeerId,
    pub udp_gossip_addr: SocketAddr,
    pub raptorq_gossip_addr: SocketAddr,
    pub kademlia_liveness_addr: SocketAddr,
}

impl Default for BootstrapPeerData {
    fn default() -> Self {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        BootstrapPeerData {
            id: KademliaPeerId::default(),
            raptorq_gossip_addr: addr,
            kademlia_liveness_addr: addr,
            udp_gossip_addr: addr,
        }
    }
}
