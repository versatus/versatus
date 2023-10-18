use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

use derive_builder::Builder;
use primitives::{KademliaPeerId, NodeId, NodeType, DEFAULT_VRRB_DATA_DIR_PATH};
use serde::Deserialize;
use uuid::Uuid;
use vrrb_core::keypair::Keypair;

use crate::{
    bootstrap::BootstrapConfig, BootstrapQuorumConfig, QuorumMember, QuorumMembershipConfig,
    ThresholdConfig,
};

#[derive(Builder, Debug, Clone, Deserialize)]
pub struct NodeConfig {
    /// UUID that identifies each node
    pub id: NodeId,

    /// Directory used to persist all VRRB node information to disk
    pub data_dir: PathBuf,

    /// Path where the database log file resides on disk
    pub db_path: PathBuf,

    /// Address the Node listens for protocol events
    pub public_ip_address: SocketAddr,

    /// ID used to identify a given node within a Kademlia DHT.
    // TODO: figure out how to merge this with id field and have kademlia-dht-rs accept it
    // TODO: make this non optional
    pub kademlia_peer_id: Option<KademliaPeerId>,

    /// Address used by Kademlia DHT listens for liveness pings
    pub kademlia_liveness_address: SocketAddr,

    /// Address the node listens for network events through udp
    pub udp_gossip_address: SocketAddr,

    /// Address the node listens for network events through RaptorQ
    pub raptorq_gossip_address: SocketAddr,

    /// This is the address that the node will use to connect to the rendezvous
    /// server.
    pub rendezvous_local_address: SocketAddr,

    /// This is the address that the node will use to connect to the rendezvous
    /// server.
    pub rendezvous_server_address: SocketAddr,

    /// The type of the node, used for custom impl's based on the type the
    /// capabilities may vary.
    // authorization so that we can have custom impl blocks based on the type.
    pub node_type: NodeType,

    /// The address each node's HTTPs server listen to connection
    pub http_api_address: SocketAddr,

    /// An optional title meant to be displayed on API docs
    pub http_api_title: String,

    /// Version meant to be displayed on API docs
    pub http_api_version: String,

    /// Optional timeout to consider when shutting down the node's HTTP API
    /// server
    pub http_api_shutdown_timeout: Option<Duration>,

    /// Address the node listens for JSON-RPC connections
    pub jsonrpc_server_address: SocketAddr,

    // TODO: refactor env-aware options
    #[builder(default = "false")]
    pub preload_mock_state: bool,

    /// Bootstrap configuration used to connect to a bootstrap node.
    pub bootstrap_config: Option<BootstrapConfig>,

    /// Non-bootstrap pre-configured quorum membership configuration
    pub quorum_config: Option<QuorumMembershipConfig>,

    /// Optional Genesis Quorum configuration used to bootstrap a new quorum
    pub bootstrap_quorum_config: Option<BootstrapQuorumConfig>,

    /// Keys used to mine blocks and sign transactions
    // TODO: rename type to more intuitive name that reflects that there's two keypairs contained
    // within this data structure
    pub keypair: Keypair,

    #[builder(default = "false")]
    /// Enables the node's reporting and control UI
    // TODO: consider renaming to enable_ui instead
    pub gui: bool,

    #[builder(default = "false")]
    /// Disables all broadcasting or listening capabilities of the node
    pub disable_networking: bool,

    #[builder(default = "false")]
    /// Enables block and transaction indexing via webhook calls to external
    /// services
    pub enable_block_indexing: bool,

    pub threshold_config: ThresholdConfig,

    pub whitelisted_nodes: Vec<QuorumMember>,
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

    pub fn merge(&self, other: NodeConfig) -> Self {
        let id = if other.id.is_empty() {
            self.id.clone()
        } else {
            other.id
        };

        Self {
            id,
            data_dir: self.data_dir.clone(),
            db_path: self.db_path.clone(),
            raptorq_gossip_address: self.raptorq_gossip_address,
            udp_gossip_address: self.udp_gossip_address,
            node_type: self.node_type,
            http_api_address: self.http_api_address,
            http_api_title: self.http_api_title.clone(),
            http_api_version: self.http_api_version.clone(),
            http_api_shutdown_timeout: self.http_api_shutdown_timeout,
            jsonrpc_server_address: self.jsonrpc_server_address,
            preload_mock_state: self.preload_mock_state,
            bootstrap_config: self.bootstrap_config.clone(),
            keypair: self.keypair.clone(),
            ..other
        }
    }
}

impl Default for NodeConfig {
    fn default() -> Self {
        let ipv4_localhost_with_random_port =
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);

        Self {
            id: Uuid::new_v4().to_string(),
            data_dir: PathBuf::from(DEFAULT_VRRB_DATA_DIR_PATH),
            db_path: PathBuf::from(DEFAULT_VRRB_DATA_DIR_PATH)
                .join("node")
                .join("db"),
            public_ip_address: ipv4_localhost_with_random_port,
            raptorq_gossip_address: ipv4_localhost_with_random_port,
            udp_gossip_address: ipv4_localhost_with_random_port,
            kademlia_peer_id: None,
            kademlia_liveness_address: ipv4_localhost_with_random_port,
            rendezvous_local_address: ipv4_localhost_with_random_port,
            rendezvous_server_address: ipv4_localhost_with_random_port,
            node_type: NodeType::Full,
            http_api_address: ipv4_localhost_with_random_port,
            http_api_title: String::from("VRRB Node"),
            http_api_version: String::from("v.0.1.0"),
            http_api_shutdown_timeout: None,
            jsonrpc_server_address: ipv4_localhost_with_random_port,
            preload_mock_state: false,
            bootstrap_config: None,
            quorum_config: None,
            bootstrap_quorum_config: None,
            keypair: Keypair::random(),
            gui: false,
            disable_networking: false,
            threshold_config: ThresholdConfig::default(),
            enable_block_indexing: false,
            whitelisted_nodes: vec![],
        }
    }
}
