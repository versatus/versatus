use config::{Config, ConfigError, File};
use node::Node;
use primitives::{
    KademliaPeerId, NodeId, NodeType, DEFAULT_VRRB_DATA_DIR_PATH, DEFAULT_VRRB_DB_PATH,
};
use serde::Deserialize;
use serde_json::{from_str as json_from_str, from_value as json_from_value, Value as JsonValue};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};
use telemetry::{error, info};
use utils::payload::digest_data_to_bytes;
use uuid::Uuid;
use vrrb_config::{NodeConfig, QuorumMember};

use crate::{
    commands::{
        keygen,
        utils::{derive_kademlia_peer_id_from_node_id, deserialize_whitelisted_quorum_members},
    },
    result::{CliError, Result},
};

const DEFAULT_OS_ASSIGNED_PORT_ADDRESS: &str = "127.0.0.1:0";
const DEFAULT_JSONRPC_ADDRESS: &str = "127.0.0.1:9293";
const DEFAULT_UDP_GOSSIP_ADDRESS: &str = DEFAULT_OS_ASSIGNED_PORT_ADDRESS;
const DEFAULT_RAPTORQ_GOSSIP_ADDRESS: &str = DEFAULT_OS_ASSIGNED_PORT_ADDRESS;
pub const GENESIS_QUORUM_SIZE: usize = 5;

#[derive(clap::Parser, Debug, Clone, Deserialize)]
pub struct RunOpts {
    /// Start node as a background process
    #[clap(short, long, action, default_value = "false")]
    pub detached: bool,

    ///Shows debugging config information
    #[clap(long, action, default_value = "false")]
    pub debug_config: bool,

    #[clap(short, long, value_parser)]
    pub id: Option<primitives::NodeId>,

    #[clap(long, value_parser)]
    pub idx: Option<primitives::NodeIdx>,

    /// Defines the type of node created by this program
    #[clap(short = 't', long, value_parser, default_value = "full")]
    pub node_type: String,

    #[clap(long, value_parser, default_value = DEFAULT_VRRB_DATA_DIR_PATH)]
    pub data_dir: PathBuf,

    #[clap(long, value_parser, default_value = DEFAULT_VRRB_DB_PATH)]
    pub db_path: PathBuf,

    #[clap(long, value_parser, default_value = DEFAULT_UDP_GOSSIP_ADDRESS)]
    pub udp_gossip_address: SocketAddr,

    #[clap(long, value_parser, default_value = DEFAULT_RAPTORQ_GOSSIP_ADDRESS)]
    pub raptorq_gossip_address: SocketAddr,

    #[clap(long, value_parser, default_value = DEFAULT_OS_ASSIGNED_PORT_ADDRESS)]
    pub http_api_address: SocketAddr,

    #[clap(long, value_parser, default_value = DEFAULT_JSONRPC_ADDRESS)]
    pub jsonrpc_api_address: SocketAddr,

    #[clap(long)]
    pub bootstrap: bool,

    #[clap(long, value_parser)]
    pub bootstrap_node_addresses: Option<Vec<SocketAddr>>,

    /// Title of the API shown on swagger docs
    #[clap(long, value_parser, default_value = "Node RPC API")]
    pub http_api_title: String,

    /// API version shown in swagger docs
    #[clap(long, value_parser, default_value = "1.0.0")]
    pub http_api_version: String,

    /// Enables the UI for the node
    #[clap(long, action, default_value = "false")]
    pub gui: bool,

    /// Disables networking capabilities of the node
    #[clap(long, action, default_value = "false")]
    pub disable_networking: bool,

    #[clap(long, value_parser ,default_value=DEFAULT_OS_ASSIGNED_PORT_ADDRESS)]
    pub rendezvous_local_address: SocketAddr,

    #[clap(long, value_parser, default_value = DEFAULT_OS_ASSIGNED_PORT_ADDRESS)]
    pub rendezvous_server_address: SocketAddr,

    #[clap(long, value_parser, default_value = DEFAULT_OS_ASSIGNED_PORT_ADDRESS)]
    pub public_ip_address: SocketAddr,

    #[clap(long)]
    pub whitelist_path: Option<String>,
}

impl From<RunOpts> for NodeConfig {
    fn from(opts: RunOpts) -> Self {
        let default_node_config = NodeConfig::default();

        let node_type = match opts.node_type.parse() {
            Ok(node_type) => node_type,
            Err(_) => default_node_config.node_type,
        };

        let http_api_title = if !opts.http_api_title.is_empty() {
            opts.http_api_title.clone()
        } else {
            default_node_config.http_api_title.clone()
        };

        Self {
            id: opts.id.unwrap_or(default_node_config.id),
            data_dir: opts.data_dir,
            db_path: opts.db_path,
            node_type,
            raptorq_gossip_address: opts.raptorq_gossip_address,
            udp_gossip_address: opts.udp_gossip_address,
            rendezvous_local_address: opts.rendezvous_local_address,
            http_api_address: opts.http_api_address,
            http_api_title,
            http_api_version: opts.http_api_version,
            http_api_shutdown_timeout: default_node_config.http_api_shutdown_timeout,
            jsonrpc_server_address: opts.jsonrpc_api_address,
            preload_mock_state: default_node_config.preload_mock_state,
            bootstrap_config: default_node_config.bootstrap_config,
            kademlia_liveness_address: default_node_config.kademlia_liveness_address,
            kademlia_peer_id: default_node_config.kademlia_peer_id,

            // TODO: avoid double key generation
            // This a random keypair gets generated here, but then afterwards we read it from disk
            // and use that if its available thus making this generation wasteful. This is a bit of
            // a hack, but it works for now.
            keypair: default_node_config.keypair,
            disable_networking: opts.disable_networking,
            gui: opts.gui,
            rendezvous_server_address: opts.rendezvous_server_address,
            public_ip_address: opts.raptorq_gossip_address,
            bootstrap_quorum_config: default_node_config.bootstrap_quorum_config,
            quorum_config: default_node_config.quorum_config,
            enable_block_indexing: default_node_config.enable_block_indexing,
            threshold_config: default_node_config.threshold_config,
            whitelisted_nodes: default_node_config.whitelisted_nodes,
        }
    }
}

impl Default for RunOpts {
    fn default() -> Self {
        let ipv4_localhost_with_random_port =
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);

        Self {
            detached: Default::default(),
            debug_config: Default::default(),
            id: Default::default(),
            idx: Default::default(),
            node_type: Default::default(),
            data_dir: Default::default(),
            db_path: Default::default(),
            udp_gossip_address: ipv4_localhost_with_random_port,
            raptorq_gossip_address: ipv4_localhost_with_random_port,
            http_api_address: ipv4_localhost_with_random_port,
            jsonrpc_api_address: ipv4_localhost_with_random_port,
            bootstrap: Default::default(),
            bootstrap_node_addresses: Default::default(),
            http_api_title: Default::default(),
            http_api_version: Default::default(),
            gui: Default::default(),
            disable_networking: Default::default(),
            rendezvous_local_address: ipv4_localhost_with_random_port,
            rendezvous_server_address: ipv4_localhost_with_random_port,
            public_ip_address: ipv4_localhost_with_random_port,
            whitelist_path: None,
        }
    }
}

impl RunOpts {
    #[deprecated(note = "prefer global config file")]
    pub fn from_file(config_path: &str) -> std::result::Result<Self, ConfigError> {
        let default_bootstrap_addresses: Vec<String> = Vec::new();

        let s = Config::builder()
            .set_default("id", Uuid::new_v4().to_string())?
            .set_default("data_dir", DEFAULT_VRRB_DATA_DIR_PATH)?
            .set_default("db_path", DEFAULT_VRRB_DB_PATH)?
            .set_default("node_type", "full")?
            .set_default("jsonrpc_api_address", DEFAULT_JSONRPC_ADDRESS)?
            .set_default("http_api_address", DEFAULT_OS_ASSIGNED_PORT_ADDRESS)?
            .set_default("http_api_title", "Node API")?
            .set_default("http_api_version", "1.0.1")?
            .set_default("bootstrap_node_addresses", default_bootstrap_addresses)?
            .set_default("preload_mock_state", false)?
            .set_default("debug_config", false)?
            .set_default("bootstrap", false)?
            .set_default("detached", false)?
            .add_source(File::with_name(config_path))
            .build()?;

        Ok(s.try_deserialize().unwrap_or_default())
    }

    pub fn merge(&self, other: &Self) -> Self {
        let node_type = match self.node_type.parse::<NodeType>() {
            Ok(_) => self.node_type.clone(),
            Err(_) => other.node_type.clone(),
        };

        let data_dir = if !self.data_dir.to_str().unwrap_or_default().is_empty() {
            self.data_dir.clone()
        } else {
            other.data_dir.clone()
        };

        let db_path = if !self.db_path.to_str().unwrap_or_default().is_empty() {
            self.db_path.clone()
        } else {
            other.db_path.clone()
        };

        let bootstrap_node_addresses = if other.bootstrap_node_addresses.is_none() {
            self.bootstrap_node_addresses.clone()
        } else {
            other.bootstrap_node_addresses.clone()
        };

        let http_api_title = if !self.http_api_title.is_empty() {
            self.http_api_title.clone()
        } else {
            other.http_api_title.clone()
        };

        let http_api_version = if !self.http_api_version.is_empty() {
            self.http_api_version.clone()
        } else {
            other.http_api_version.clone()
        };

        Self {
            detached: other.detached,
            debug_config: other.debug_config,
            id: self.id.clone().or(other.id.clone()),
            idx: self.idx.or(other.idx),
            node_type,
            data_dir,
            db_path,
            // TODO: reconsider override strategies
            udp_gossip_address: other.udp_gossip_address,
            raptorq_gossip_address: other.raptorq_gossip_address,
            jsonrpc_api_address: other.jsonrpc_api_address,
            bootstrap: other.bootstrap,
            bootstrap_node_addresses,
            http_api_address: other.http_api_address,
            http_api_title,
            http_api_version,
            gui: false,
            disable_networking: false,
            rendezvous_local_address: other.rendezvous_local_address,
            rendezvous_server_address: other.rendezvous_server_address,
            public_ip_address: other.public_ip_address,
            whitelist_path: other.whitelist_path.clone(),
        }
    }
}

/// Configures and runs a VRRB Node
pub async fn run(args: RunOpts) -> Result<()> {
    let keypair = keygen::keygen(false)?;

    let mut node_config = NodeConfig::from(args.clone());
    node_config.keypair = keypair;

    let derived_kademlia_peer_id = derive_kademlia_peer_id_from_node_id(&node_config.id)?;
    node_config.kademlia_peer_id = Some(derived_kademlia_peer_id);

    // TODO: prevent parsing errors when no kademlia peer id is present
    let mut whitelisted_nodes = args
        .whitelist_path
        .and_then(|whitelist| {
            let mut finalized_whitelist = Vec::with_capacity(GENESIS_QUORUM_SIZE);
            if let Err(e) =
                deserialize_whitelisted_quorum_members(whitelist, &mut finalized_whitelist)
            {
                error!("failed to deserialize whitelist: {e}");
            }
            Some(finalized_whitelist)
        })
        .unwrap_or_default();

    whitelisted_nodes
        .iter_mut()
        .try_for_each(|member| -> Result<()> {
            member.kademlia_peer_id = derive_kademlia_peer_id_from_node_id(&member.node_id)?;
            Ok(())
        })?;

    node_config.whitelisted_nodes = whitelisted_nodes;

    if args.debug_config {
        dbg!(&node_config);
    }

    if args.detached {
        run_detached(node_config).await
    } else {
        run_blocking(node_config).await
    }
}

#[telemetry::instrument]
async fn run_blocking(node_config: NodeConfig) -> Result<()> {
    let vrrb_node = Node::start(node_config)
        .await
        .map_err(|err| CliError::Other(err.to_string()))?;

    let node_type = vrrb_node.node_type();

    info!("running {node_type:?} node in blocking mode");

    tokio::signal::ctrl_c()
        .await
        .map_err(|err| CliError::Other(format!("failed to listen for ctrl+c: {err}")))?;

    vrrb_node.stop().await?;

    info!("Node stopped");

    Ok(())
}

#[telemetry::instrument]
async fn run_detached(node_config: NodeConfig) -> Result<()> {
    info!("running node in detached mode");
    // start child process, run node within it
    Ok(())
}
