use clap::{Parser, Subcommand};

use crate::commands::{describe::DescribeOpts, execute::ExecuteOpts, validate::ValidateOpts};

#[derive(Parser)]
#[clap(author, version, about)]
pub struct WasmCli {
    /// CLI subcommand
    #[clap(subcommand)]
    pub cmd: Option<WasmCommands>,
}

#[derive(Subcommand)]
pub enum WasmCommands {
    /// Describes details about a WASM module
    Describe(DescribeOpts),
    /// Execute a Web Assembly module
    Execute(ExecuteOpts),
    /// Validates a WASM module's ability to execute
    Validate(ValidateOpts),
}
