use crate::result::{CliError, Result};
use clap::{Parser, Subcommand};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum NodeType {
    /// A Node that can archive, validate and mine tokens
    Full,
    /// Same as `NodeType::Full` but without archiving capabilities
    Light,
    /// Archives all transactions processed in the blockchain
    Archive,
    /// Mining node
    Miner,
    Bootstrap,
    Validator,
}

#[derive(Debug, Error)]
pub enum NodeCliError {
    #[error("invalid node type {0} provided")]
    InvalidNodeType(String),

    #[error("unable to setup telemetry subscriber: {0}")]
    Telemetry(#[from] telemetry::TelemetryError),

    #[error("node runtime error: {0}")]
    Service(#[from] runtime::RuntimeError),
}

impl FromStr for NodeType {
    type Err = NodeCliError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // TODO: define node types thoroughly
        match s {
            "full" => Ok(NodeType::Full),
            "light" => Ok(NodeType::Light),
            _ => Err(NodeCliError::InvalidNodeType(s.into())),
        }
    }
}

#[derive(clap::Parser, Debug)]
pub struct RunOpts {
    /// node_type defines the type of node created by this program
    #[clap(short, long, value_parser, default_value = "full")]
    pub node_type: String,
}

#[derive(Debug, Subcommand)]
pub enum NodeCmd {
    /// Run a node with the provided configuration
    Run(RunOpts),
}

#[derive(clap::Parser, Debug)]
pub struct NodeOpts {
    #[clap(subcommand)]
    pub subcommand: NodeCmd,
}

pub async fn exec(args: NodeOpts) -> Result<()> {
    telemetry::debug!("args: {:?}", args);

    let sub_cmd = args.subcommand;

    match sub_cmd {
        NodeCmd::Run(opts) => run(opts).await,
        _ => Err(CliError::InvalidCommand(format!("{:?}", sub_cmd))),
    }
}

pub async fn run(args: RunOpts) -> Result<()> {
    telemetry::debug!("args: {:?}", args);

    let node_type = args.node_type;

    telemetry::info!("creating {}", node_type);

    let runtime_opts = runtime::RuntimeOpts { node_type };

    let node_runtime = runtime::Runtime::new();

    node_runtime.start(runtime_opts).await?;

    Ok(())
}
