mod info;
mod run;

use clap::{Parser, Subcommand};

pub use run::*;

use crate::result::{CliError, Result};

#[derive(Debug, Subcommand)]
pub enum DevCmd {
    /// Run a node with the provided configuration
    Run(Box<RunOpts>),

    /// Prints current node configuration
    Info,

    /// Stops any node currently running in detached mode
    Stop,
}

#[derive(Parser, Debug)]
pub struct DevOpts {
    #[clap(subcommand)]
    pub subcommand: DevCmd,
}

pub async fn exec(args: DevOpts) -> Result<()> {
    let sub_cmd = args.subcommand;

    match sub_cmd {
        DevCmd::Run(opts) => run(*opts).await,
        DevCmd::Info => Ok(()),
        _ => Err(CliError::InvalidCommand(format!("{sub_cmd:?}"))),
    }
}
