use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

use clap::{Parser, Subcommand};
use node::Node;
use uuid::Uuid;
use vrrb_config::NodeConfig;
use vrrb_core::event_router::Event;

use crate::result::{CliError, Result};

#[derive(clap::Parser, Debug)]
pub struct RunOpts {
    /// Start node as a background process
    #[clap(short, long, action)]
    pub dettached: bool,

    #[clap(short, long, value_parser)]
    pub id: primitives::types::NodeId,

    #[clap(long, value_parser)]
    // TODO: reconsider this id
    pub node_idx: primitives::types::NodeIdx,

    /// Defines the type of node created by this program
    #[clap(short = 't', long, value_parser, default_value = "full")]
    pub node_type: String,

    #[clap(long, value_parser)]
    pub data_dir: PathBuf,

    #[clap(long, value_parser)]
    pub db_path: PathBuf,

    #[clap(long, value_parser)]
    pub gossip_address: SocketAddr,

    #[clap(long)]
    pub bootstrap: bool,

    #[clap(long, value_parser)]
    pub bootstrap_node_addresses: Vec<SocketAddr>,
}

#[derive(Debug, Subcommand)]
pub enum NodeCmd {
    /// Run a node with the provided configuration
    Run(RunOpts),

    /// Stops any currently running node if any
    Stop,
}

#[derive(Parser, Debug)]
pub struct NodeOpts {
    #[clap(subcommand)]
    pub subcommand: NodeCmd,
}

pub async fn exec(args: NodeOpts) -> Result<()> {
    let sub_cmd = args.subcommand;

    match sub_cmd {
        NodeCmd::Run(opts) => run(opts).await,
        _ => Err(CliError::InvalidCommand(format!("{:?}", sub_cmd))),
    }
}

/// Configures and runs a VRRB Node
pub async fn run(args: RunOpts) -> Result<()> {
    // TODO: get these from proper config
    let id = Uuid::new_v4().to_simple().to_string();
    let idx = args.node_idx;
    let node_type = args.node_type.parse()?;
    let data_dir = storage::get_node_data_dir()?;
    let db_path = args.db_path;
    let bootstrap = args.bootstrap;
    let bootstrap_node_addresses = args.bootstrap_node_addresses;

    let gossip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let http_api_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000);

    let http_api_title = String::from("Node HTTP API");
    let http_api_version = String::from("1.0.0");
    let http_api_shutdown_timeout = Some(Duration::from_secs(5));

    let node_config = NodeConfig {
        id,
        idx,
        data_dir,
        node_type,
        db_path,
        gossip_address,
        bootstrap,
        bootstrap_node_addresses,
        http_api_address,
        http_api_title,
        http_api_version,
        http_api_shutdown_timeout,
    };

    if args.dettached {
        run_dettached(node_config).await
    } else {
        run_blocking(node_config).await
    }
}

#[telemetry::instrument]
async fn run_blocking(node_config: NodeConfig) -> Result<()> {
    let (_ctrl_tx, mut ctrl_rx) = tokio::sync::mpsc::unbounded_channel::<Event>();

    let mut vrrb_node = Node::new(node_config);

    telemetry::info!("running node in blocking mode");

    vrrb_node.start(&mut ctrl_rx).await?;

    Ok(())
}

#[telemetry::instrument]
async fn run_dettached(node_config: NodeConfig) -> Result<()> {
    telemetry::info!("running node in dettached mode");
    // start child process, run node within it
    Ok(())
}
