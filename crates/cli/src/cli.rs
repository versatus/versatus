use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::commands::node::NodeOpts;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Sets a custom config file
    #[clap(short, long, value_parser, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Turn debugging information on
    #[clap(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Node management subcommands
    Node(NodeOpts),

    /// Wallet management subcommands
    Wallet(NodeOpts),
}
