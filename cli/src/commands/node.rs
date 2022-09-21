use std::path::PathBuf;

use clap::{Parser, Subcommand};
use commands::command::Command;
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

/// Configures and runs a VRRB Node
pub async fn run(args: RunOpts) -> Result<()> {
    let node_type = args.node_type.parse()?;
    let data_dir = storage::get_node_data_dir()?;

    let rt_opts = RuntimeOpts {
        node_type,
        data_dir,
        node_idx: 100,
    };

    if args.dettached {
        run_dettached(rt_opts).await
    } else {
        run_blocking(rt_opts).await
    }
}

#[telemetry::instrument]
async fn run_blocking(rt_opts: RuntimeOpts) -> Result<()> {
    let (ctrl_tx, ctrl_rx) = tokio::sync::mpsc::unbounded_channel::<Command>();

    let mut node_runtime = runtime::Runtime::new(ctrl_rx);

    telemetry::info!("running node in blocking mode");

    node_runtime.start(rt_opts).await?;

    Ok(())
}

#[telemetry::instrument]
async fn run_dettached(rt_opts: RuntimeOpts) -> Result<()> {
    telemetry::info!("running node in dettached mode");
    // start child process, run node within it
    Ok(())
}
