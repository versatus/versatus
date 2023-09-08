use clap::{Parser, Subcommand};

use crate::commands::daemon::DaemonOpts;
use crate::commands::status::StatusOpts;

#[derive(Parser)]
#[clap(author, version, about)]
pub struct ComputeCli {
    /// The path to the service configuration JSON file.
    #[clap(
        short,
        long,
        value_parser,
        value_name = "FILENAME",
        default_value = "./services.json"
    )]
    pub config: String,
    /// The name of the compute service name.
    #[clap(
        short,
        long,
        value_parser,
        value_name = "SERVICE",
        default_value = "default"
    )]
    pub service: String,
    /// The type of service definition to look up.
    #[clap(short='t', long, value_parser, value_name = "TYPE")]
    pub service_type: Option<String>,
    /// CLI subcommand
    #[clap(subcommand)]
    pub cmd: Option<ComputeCommands>,
}

#[derive(Subcommand)]
pub enum ComputeCommands {
    /// Starts the compute agent daemon
    Daemon(DaemonOpts),
    /// Shows status of a running agent
    Status(StatusOpts),
}
