use derive_builder::Builder;
use secp256k1::{PublicKey, SecretKey};
use std::{net::SocketAddr, path::PathBuf, time::Duration};

mod bootstrap;

#[derive(Builder, Debug, Clone)]
pub struct NodeConfig {
    pub id: primitives::NodeId,
    pub idx: primitives::NodeIdx,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub raptorq_gossip_address: SocketAddr,
    pub udp_gossip_address: SocketAddr,
    pub node_type: primitives::NodeType,
    pub bootstrap_node_addresses: Vec<SocketAddr>,
    pub http_api_address: SocketAddr,
    pub http_api_title: String,
    pub http_api_version: String,
    pub http_api_shutdown_timeout: Option<Duration>,

    // TODO: refactor env-aware options
    #[builder(default = "false")]
    pub preload_mock_state: bool,

    //
    //TODO: use SecretKey from threshold crypto crate for MasterNode
    //TODO: Discussion :Generation/Serializing/Deserialzing of secret key to be
    // moved to primitive/utils module
    // let mut secret_key_encoded = Vec::new();
    //
    // TODO: replace keys with hhbft ones
    pub node_public_key: PublicKey,
    pub node_secret_key: SecretKey,

    /// Address the node listens for JSON-RPC connections
    pub jsonrpc_server_address: SocketAddr,
}

impl NodeConfig {
    pub fn db_path(&self) -> &PathBuf {
        // TODO: refactor to Option and check if present and return configured db path
        // or default path within vrrb's data dir
        &self.db_path
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use primitives::NodeType;
    use secp256k1::Secp256k1;

    use super::*;

    #[test]
    fn can_be_built_using_a_builder() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);

        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
        let (secret_key, pubkey) = secp.generate_keypair(&mut rng);

        NodeConfigBuilder::default()
            .id(String::from("abcdefg"))
            .idx(10)
            .data_dir("mock_path".into())
            .db_path("mock_path".into())
            .raptorq_gossip_address(addr)
            .udp_gossip_address(addr)
            .http_api_address(addr)
            .jsonrpc_server_address(addr)
            .http_api_title(String::from("mock title"))
            .http_api_version(String::from("1.0"))
            .http_api_shutdown_timeout(None)
            .node_type(NodeType::Full)
            .bootstrap_node_addresses(vec![addr])
            .node_public_key(pubkey)
            .node_secret_key(secret_key)
            .build()
            .unwrap();
    }
}
