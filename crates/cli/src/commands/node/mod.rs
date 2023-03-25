mod info;
mod run;

use clap::{Parser, Subcommand};
pub use info::*;
pub use run::*;

use crate::result::{CliError, Result};

#[derive(Debug, Subcommand)]
pub enum NodeCmd {
    /// Run a node with the provided configuration
    Run(RunOpts),

    /// Prints currrent node configuration
    Info,

    /// Stops any node currrently running in dettached mode
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
        NodeCmd::Info => info::exec().await,
        _ => Err(CliError::InvalidCommand(format!("{:?}", sub_cmd))),
    }
}
