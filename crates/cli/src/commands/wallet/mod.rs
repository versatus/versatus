use clap::{Parser, Subcommand};

#[derive(Debug, Subcommand)]
pub enum WalletCmd {
    /// Transfer objects between accounts
    Transfer,

    /// Create a new account on the network
    New,

    /// Gets information about an account
    Get,
}

#[derive(Parser, Debug)]
pub struct WalletOpts {
    #[clap(subcommand)]
    pub subcommand: WalletCmd,
}
