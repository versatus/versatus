use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use commands::command::Command;
use vrrb_core::event_router::{Event, Topic, EventRouter, DirectedEvent};
use node::{Node, NodeType};
use tokio::sync::oneshot;
use uuid::Uuid;
use vrrb_config::NodeConfig;

use crate::result::{CliError, Result};

#[derive(clap::Parser, Debug)]
pub struct RunOpts {
    /// Start node as a background process
    #[clap(short, long, action)]
    pub dettached: bool,

    #[clap(short, long, value_parser)]
    pub id: primitives::NodeId,

    #[clap(long, value_parser)]
    // TODO: reconsider this id
    pub node_idx: primitives::NodeIdx,

    /// Defines the type of node created by this program
    #[clap(short = 't', long, value_parser, default_value = "full")]
    pub node_type: String,

    #[clap(long, value_parser)]
    pub data_dir: PathBuf,

    #[clap(long, value_parser)]
    pub db_path: PathBuf,

    #[clap(long, value_parser)]
    pub address: SocketAddr,

    #[clap(long)]
    pub bootstrap: bool,

    #[clap(long, value_parser)]
    pub bootstrap_node_addr: SocketAddr,
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
    let node_type = args.node_type.parse()?;
    let data_dir = storage::get_node_data_dir()?;
    let node_idx = args.node_idx;
    let db_path = args.db_path;
    let bootstrap = args.bootstrap;
    let bootstrap_node_addr = args.bootstrap_node_addr;

    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    let id = Uuid::new_v4().to_simple().to_string();
    let idx = 100;

    let node_config = NodeConfig {
        id,
        idx,
        data_dir,
        node_type,
        db_path,
        node_idx,
        bootstrap,
        address,
        bootstrap_node_addr: address,
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

    vrrb_node.start(&mut ctrl_rx).await?;

    Ok(())
}

#[telemetry::instrument]
async fn run_dettached(node_config: NodeConfig) -> Result<()> {
    telemetry::info!("running node in dettached mode");
    // start child process, run node within it
    Ok(())
}
