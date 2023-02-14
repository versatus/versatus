use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::commands::{node::NodeOpts, wallet::WalletOpts};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None, arg_required_else_help(true))]
pub struct Args {
    /// Sets a custom config file
    #[clap(short, long, value_parser, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Turn debugging information on
    #[clap(short, long)]
    pub debug: u8,

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Manage configuration for this CLI tool
    Config,

    /// Interact with and control VRRB nodes
    Node(NodeOpts),

    /// Interact with with accounts and objects on the network
    Wallet(WalletOpts),
}
