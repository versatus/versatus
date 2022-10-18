// use secp256k1::{PublicKey, Secp256k1, SecretKey};
use std::{net::SocketAddr, path::PathBuf};

#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub id: primitives::types::NodeId,
    pub idx: primitives::types::NodeIdx,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub node_idx: primitives::types::NodeIdx,
    pub address: SocketAddr,
    pub bootstrap: bool,
    pub bootstrap_node_addr: SocketAddr,
    pub node_type: primitives::types::NodeType,
    // pub public_key: primitives::PublicKey,
    // pub secret_key: primitives::SecretKey,
}

impl NodeConfig {
    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
