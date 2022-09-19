use std::path::PathBuf;

use clap::{Parser, Subcommand};
use node::core::NodeType;
use runtime::RuntimeOpts;
use tokio::sync::oneshot;

use crate::result::{CliError, Result};

#[derive(clap::Parser, Debug)]
pub struct RunOpts {
    /// Defines the type of node created by this program
    #[clap(short, long, value_parser, default_value = "full")]
    pub node_type: String,

    /// Start node as a background process
    #[clap(short, long, action)]
    pub dettached: bool,
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

pub async fn run(args: RunOpts) -> Result<()> {
    telemetry::debug!("args: {:?}", args);

    let node_type = args.node_type.parse()?;

    telemetry::info!("creating {:?}", node_type);

    let (ctrl_tx, ctrl_rx) = oneshot::channel();

    let rt_opts = RuntimeOpts {
        node_type,
        data_dir: PathBuf::from("/tmp/vrrb"),
        node_idx: 100,
    };

    let mut node_runtime = runtime::Runtime::new(ctrl_rx);

    node_runtime.start(rt_opts).await?;

    Ok(())
}
