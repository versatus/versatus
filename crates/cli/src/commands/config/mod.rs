use clap::{Parser, Subcommand};

#[derive(Debug, Subcommand)]
pub enum ConfigCmd {
    /// Prints CLI  configuration
    Info,

    /// Gets a specific value from the configuration file
    Get,

    /// Assigns a value to a specific key in the configuration file
    Set,

    /// Removes all data within VRRB's data directory
    Clean,
}

#[derive(Parser, Debug)]
pub struct ConfigOpts {
    #[clap(subcommand)]
    pub subcommand: ConfigCmd,
}
