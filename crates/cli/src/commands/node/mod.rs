mod info;
mod run;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

use clap::{Parser, Subcommand};
use config::{Config, ConfigError, File};
use hbbft::crypto::{serde_impl::SerdeSecret, PublicKey, SecretKey};
pub use info::*;
use node::{Node, NodeType};
use primitives::{DEFAULT_VRRB_DATA_DIR_PATH, DEFAULT_VRRB_DB_PATH};
pub use run::*;
use secp256k1::{rand, Secp256k1};
use serde::Deserialize;
use telemetry::{error, info};
use uuid::Uuid;
use vrrb_config::NodeConfig;
use vrrb_core::{
    event_router::Event,
    keypair::{self, read_keypair_file, write_keypair_file, Keypair},
};

use crate::result::{CliError, Result};


const DEFAULT_OS_ASSIGNED_PORT_ADDRESS: &'static str = "127.0.0.1:0";

#[derive(clap::Parser, Debug, Clone, Deserialize)]
pub struct RunOpts {
    /// Start node as a background process
    #[clap(short, long, action, default_value = "false")]
    pub dettached: bool,

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

    #[clap(long, value_parser, default_value = DEFAULT_OS_ASSIGNED_PORT_ADDRESS)]
    pub udp_gossip_address: SocketAddr,

    #[clap(long, value_parser, default_value = DEFAULT_OS_ASSIGNED_PORT_ADDRESS)]
    pub raptorq_gossip_address: SocketAddr,

    #[clap(long, value_parser, default_value = DEFAULT_OS_ASSIGNED_PORT_ADDRESS)]
    pub http_api_address: SocketAddr,

    #[clap(long, value_parser, default_value = "127.0.0.1:0")]
    pub jsonrpc_api_address: SocketAddr,

    #[clap(long, default_value = "false")]
    pub bootstrap: bool,

    #[clap(long, value_parser)]
    pub bootstrap_node_addresses: Option<Vec<SocketAddr>>,

    /// Title of the API shown on swagger docs
    #[clap(long, value_parser, default_value = "Node RPC API")]
    pub http_api_title: String,

    /// API version shown in swagger docs
    #[clap(long, value_parser, default_value = "1.0.0")]
    pub http_api_version: String,
}

impl From<RunOpts> for NodeConfig {
    fn from(opts: RunOpts) -> Self {
        let default_node_config = NodeConfig::default();

        let node_type = match opts.node_type.parse() {
            Ok(node_type) => node_type,
            Err(_) => default_node_config.node_type.clone(),
        };

        let http_api_title = if !opts.http_api_title.is_empty() {
            opts.http_api_title.clone()
        } else {
            default_node_config.http_api_title.clone()
        };


        Self {
            id: opts.id.unwrap_or(default_node_config.id),
            idx: opts.idx.unwrap_or(default_node_config.idx),
            data_dir: opts.data_dir,
            db_path: opts.db_path,
            node_type,
            raptorq_gossip_address: opts.raptorq_gossip_address,
            udp_gossip_address: opts.udp_gossip_address,
            http_api_address: opts.http_api_address,
            http_api_title,
            http_api_version: opts.http_api_version,
            http_api_shutdown_timeout: default_node_config.http_api_shutdown_timeout,
            jsonrpc_server_address: opts.jsonrpc_api_address,
            preload_mock_state: default_node_config.preload_mock_state,
            bootstrap_config: default_node_config.bootstrap_config,
            bootstrap_node_addresses: opts
                .bootstrap_node_addresses
                .unwrap_or(default_node_config.bootstrap_node_addresses),

            // TODO: avoid double key generation
            // This a random keypair gets generated here, but then afterwards we read it from disk
            // and use that if its available thus making this generation wasteful. This is a bit of
            // a hack, but it works for now.
            keypair: default_node_config.keypair,
        }
    }
}

impl Default for RunOpts {
    fn default() -> Self {
        let ipv4_localhost_with_random_port =
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);

        Self {
            dettached: Default::default(),
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
        }
    }
}


impl RunOpts {
    pub fn from_file(config_path: &str) -> std::result::Result<Self, ConfigError> {
        let default_bootstrap_addresses: Vec<String> = Vec::new();

        let s = Config::builder()
            .set_default("id", Uuid::new_v4().to_string())?
            .set_default("data_dir", ".vrrb")?
            .set_default("db_path", ".vrrb/node/node.db")?
            .set_default("node_type", "full")?
            .set_default("jsonrpc_api_address", "127.0.0.1:0")?
            .set_default("http_api_address", "127.0.0.1:0")?
            .set_default("http_api_title", "Node API")?
            .set_default("http_api_version", "1.0.1")?
            .set_default("bootstrap_node_addresses", default_bootstrap_addresses)?
            .set_default("preload_mock_state", false)?
            .set_default("debug_config", false)?
            .set_default("bootstrap", false)?
            .set_default("dettached", false)?
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
            dettached: other.dettached,
            debug_config: other.debug_config,
            id: self.id.clone().or(other.id.clone()),
            idx: self.idx.clone().or(other.idx),
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
        }
    }
}


#[derive(Debug, Subcommand)]
pub enum NodeCmd {
    /// Run a node with the provided configuration
    Run(RunOpts),

    /// Prints currrent node configuration
    Info,

    /// Stops any currently running node if any
    Stop,
}

#[derive(Parser, Debug)]
pub struct NodeOpts {
    /// Sets a custom config file
    #[clap(short, long, value_parser, value_name = "FILE")]
    pub config: Option<PathBuf>,

    #[clap(subcommand)]
    pub subcommand: NodeCmd,
}

pub async fn exec(args: NodeOpts) -> Result<()> {
    let sub_cmd = args.subcommand;

    match sub_cmd {
        NodeCmd::Run(opts) => {
            let read_opts = read_node_config_from_file(args.config.unwrap_or_default())?;

            let merged_opts = read_opts.merge(&opts);

            run(merged_opts).await
        },
        NodeCmd::Info => {
            if let Some(config_file_path) = args.config {
                let node_config = read_node_config_from_file(config_file_path)?;

                dbg!(node_config);
            }

            Ok(())
        },
        _ => Err(CliError::InvalidCommand(format!("{:?}", sub_cmd))),
    }
}

pub fn read_node_config_from_file(config_file_path: PathBuf) -> Result<RunOpts> {
    let path_str = config_file_path.to_str().unwrap_or_default();

    let node_config = RunOpts::from_file(path_str)
        .map_err(|err| CliError::Other(format!("failed to read config file: {err}")))?;

    Ok(node_config)
}
