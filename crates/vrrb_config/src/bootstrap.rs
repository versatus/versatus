use std::net::SocketAddr;

use serde::Deserialize;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct BootstrapConfig {
    /// List of known peers to bootstrap the network with.
    pub known_trusted_peers: Vec<SocketAddr>,
}
