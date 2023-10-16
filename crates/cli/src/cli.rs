use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::commands::{config::ConfigOpts, keygen::KeygenCmd, node::NodeOpts, wallet::WalletOpts};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None, arg_required_else_help(true))]
pub struct Args {
    /// Sets a custom config file
    #[clap(short, long, value_parser, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Turn debugging information on
    #[clap(short, long, default_value = "false")]
    pub debug: bool,

    /// Turn debugging information on
    #[clap(short, long, default_value = "local")]
    pub network: String,

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Manage configuration for this CLI tool
    Config(ConfigOpts),

    /// Interact with and control VRRB nodes
    Node(Box<NodeOpts>),

    /// Interact with with accounts and objects on the network
    Wallet(WalletOpts),

    /// Manage keypair creation
    Keygen(KeygenCmd),
}
