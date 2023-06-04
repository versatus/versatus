use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct BootstrapConfig {
    pub addresses: Vec<SocketAddr>,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        BootstrapConfig { addresses: vec![] }
    }
}
