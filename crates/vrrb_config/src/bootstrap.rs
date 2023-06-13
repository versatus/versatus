use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::NodeId;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct BootstrapConfig {
    pub id: NodeId,
    pub addr: SocketAddr,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        BootstrapConfig {
            id: Uuid::nil().to_string(),
            addr,
        }
    }
}
