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
    pub idx: primitives::types::NodeIdx,

    /// Defines the type of node created by this program
    #[clap(short = 't', long, value_parser, default_value = "full")]
    pub node_type: String,

    #[clap(long, value_parser)]
    pub data_dir: PathBuf,

    #[clap(long, value_parser)]
    pub db_path: PathBuf,

    #[clap(long, value_parser)]
    pub gossip_address: SocketAddr,

    #[clap(long, value_parser)]
    pub http_api_address: SocketAddr,

    #[clap(long)]
    pub bootstrap: bool,

    #[clap(long, value_parser)]
    pub bootstrap_node_addresses: Vec<SocketAddr>,

    /// Title of the API shown on swagger docs
    #[clap(long, value_parser, default_value = "Node RPC API")]
    pub http_api_title: String,

    /// API version shown in swagger docs
    #[clap(long, value_parser)]
    pub http_api_version: String,
    //
    // /// API shutdown timeout
    // #[clap(long)]
    // pub http_api_shutdown_timeout: Option<Duration>,
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
    let idx = args.idx;
    let node_type = args.node_type.parse()?;
    let data_dir = storage::get_node_data_dir()?;
    let db_path = args.db_path;
    let bootstrap = args.bootstrap;
    let bootstrap_node_addresses = args.bootstrap_node_addresses;

    let gossip_address = args.gossip_address;
    let http_api_address = args.http_api_address;

    let http_api_title = args.http_api_title;
    let http_api_version = args.http_api_version;
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
    let (ctrl_tx, mut ctrl_rx) = tokio::sync::mpsc::unbounded_channel::<Event>();

    let mut vrrb_node = Node::new(node_config);

    telemetry::info!("running node in blocking mode");

    let node_handle = tokio::spawn(async move {
        // NOTE: starts the main node service
        vrrb_node.start(&mut ctrl_rx).await
    });

    tokio::signal::ctrl_c()
        .await
        .map_err(|_| CliError::Other(String::from("failed to listen for ctrl+c")))?;

    ctrl_tx
        .send(Event::Stop)
        .map_err(|_| CliError::Other(String::from("failed to send stop event to node")))?;

    node_handle
        .await
        .map_err(|_| CliError::Other(String::from("failed to join node task handle")))??;

    Ok(())
}

#[telemetry::instrument]
async fn run_dettached(node_config: NodeConfig) -> Result<()> {
    telemetry::info!("running node in dettached mode");
    // start child process, run node within it
    Ok(())
}
