use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct InitTestDBOpts {
    /// This is the path to the database to be created/used. #716, this path is what we'll feed
    /// into the database driver.
    #[clap(short, long)]
    pub dbpath: String,
    /// Force DB to be initialised, even if it already exists. #716 I think that if this option is
    /// missing or false, we should only recreate the database from defaults if it doesn't already
    /// exist. If it exists, we should exit with a failure and an error message indicating that the
    /// database already exists and to use --force.
    #[clap(short, long)]
    pub force: Option<bool>,
    /// Default balance for new test accounts created. The protocol supports values up to
    /// [ethnum::U256] in size, but u128 ought to be fine for now.
    #[clap(short, long)]
    pub default_balance: Option<u128>,
}

/// Initialises a new database for keeping standalone state typically provided by a blockchain.
/// This allows some standalone testing of smart contracts without needing access to a testnet and
/// can also potentially be integrated into common CI/CD frameworks.
pub fn run(opts: &InitTestDBOpts) -> Result<()> {
    // #716, here we want to create a new database to be used by the rest of the functionality in
    // issue #716. This database could be SQLite3 or similar, but with some caveats:
    //  - The database can be written to a single file
    //  - The database has native Rust drivers (ie, not any kind of C/FFI dependency)
    //  - No other external dependencies (such as a specific binary or library to have to be
    //  installed in order to run)
    //  - Not require a separate database service to be running in the background.
    //  - Hopefully be able to support U256 integers
    //
    //  Given these options, I *believe* that these might be suitable:
    //  * https://github.com/rusqlite/rusqlite
    //  * https://www.structsy.rs/
    //
    //  There may be others too. My guess is that rusqlite is probably going to be the most ideal.
    //
    //  [actually, it looks like rusqlite has a dependency on libsqlite, which
    //  isn't necessarily present on the machines we want to run on. I took a
    //  quick look at BonsaiDB and it looks like a pretty good fit for what we
    //  want. It's not a single file, but does contain everything under a single
    //  directory, which ought to suffice.
    //
    //  When creating the new database, I think we want two tables:
    //
    //  1) accounts, which is a two-column table containing a column for an account address, and a
    //     column for an account balance (to mirror this struct https://github.com/versatus/versatus-rust/blob/main/src/versatus_rust.rs#L83). When creating this table, we should also create 16 sample accounts with the addresses 0x000....[1-f].
    //     We should also assign each a default balance -- either the one specified on the command
    //     line (see option above) or, say, 1000.
    //
    //  2) protocol, which is a two column table with a single row that represents the protocol
    //     inputs struct (https://github.com/versatus/versatus-rust/blob/main/src/versatus_rust.rs#L70).
    //     We need only track the block_height (monotonically incrementing number) and the block time (date stamp).
    //
    //     Anytime the test subcommand is executed, these two fields should be updated. I'll
    //     include details under that subcommand's code.
    Ok(())
}
