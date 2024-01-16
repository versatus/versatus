use clap::{Parser, Subcommand};

use crate::commands::pkginfo::FetchMetadataOpts;
use crate::commands::{
    describe::DescribeOpts, execute::ExecuteOpts, publish::PublishOpts,
    testbalance::TestBalanceOpts, testcontract::TestContractOpts, testinitdb::TestInitDBOpts,
    validate::ValidateOpts,
};

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
    /// Initialise a test database
    TestInitDB(TestInitDBOpts),
    /// A test to check the balance of a given account
    TestBalance(TestBalanceOpts),
    /// A subcommand for executing/testing a smart contract function.
    TestContract(TestContractOpts),
    /// Publishes a smart contract package to the network
    Publish(PublishOpts),
    /// Fetching metadata about smart contract package from the network
    PkgInfo(FetchMetadataOpts),
}
