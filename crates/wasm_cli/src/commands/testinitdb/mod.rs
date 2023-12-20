use anyhow::Result;
use bonsaidb::local::Storage;
use bonsaidb::{
    core::schema::SerializedCollection,
    local::config::{Builder, StorageConfiguration},
};
use bonsaidb_core::connection::{Connection, StorageConnection};
use bonsaidb_core::schema::Collection;
use clap::Parser;
use ethereum_types::U256;
use primitives::Address;
use serde::{Deserialize, Serialize};

const DEFAULT_BALANCE: U256 = U256([10000; 4]);
const DEFAULT_ADDRESSES: &[Address; 10] = &[
    Address([0; 20]),
    Address([1; 20]),
    Address([2; 20]),
    Address([3; 20]),
    Address([4; 20]),
    Address([5; 20]),
    Address([6; 20]),
    Address([7; 20]),
    Address([8; 20]),
    Address([9; 20]),
];

// #[derive(Debug, Schema)]
// #[schema(name = "db-schema", collections = [AccountInfo, ProtocolInputs ])]
// struct DBSchema;

#[derive(Clone, Parser, Debug)]
pub struct TestInitDBOpts {
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
    pub default_balance: Option<U256>,
}

#[derive(Collection, Serialize, Deserialize, Clone, Parser, Debug)]
#[collection(name = "account-info")]
pub struct AccountInfo {
    /// Address of the smart contract's blockchain account
    pub account_address: Address,
    /// Current balance of the smart contract's account at last block
    pub account_balance: U256,
}

#[derive(Collection, Serialize, Deserialize, Clone, Parser, Debug)]
#[collection(name = "protocol-inputs")]
pub struct ProtocolInputs {
    /// The block number/height of the block currently being processed
    pub block_height: u64,
    /// The timestamp of the block currently being processed
    pub block_time: u64,
}

// Template for inserting information via connection. Will need to do so to get mock information
// to be stored in the two tables requested.
fn insert_account_info<C: Connection>(
    connection: &C,
    account_address: Address,
    account_balance: U256,
) -> Result<(), bonsaidb::core::Error> {
    AccountInfo {
        account_address,
        account_balance,
    }
    .push_into(connection)?;
    Ok(())
}

fn insert_meta_data<C: Connection>(
    connection: &C,
    block_height: u64,
    block_time: u64,
) -> Result<(), bonsaidb::core::Error> {
    ProtocolInputs {
        block_height,
        block_time,
    }
    .push_into(connection)?;
    Ok(())
}

/// Initialises a new database for keeping standalone state typically provided by a blockchain.
/// This allows some standalone testing of smart contracts without needing access to a testnet and
/// can also potentially be integrated into common CI/CD frameworks.
pub fn run(opts: &TestInitDBOpts) -> Result<()> {
    let storage = Storage::open(
        StorageConfiguration::new(&opts.dbpath)
            .with_schema::<AccountInfo>()?
            .with_schema::<ProtocolInputs>()?,
    )?;

    storage.create_database::<AccountInfo>("account-info", true)?;
    let account_info = storage.database::<AccountInfo>("account-info")?;
    storage.create_database::<ProtocolInputs>("protocol-inputs", true)?;
    let protocol_inputs = storage.database::<ProtocolInputs>("protocol-inputs")?;

    for i in 0..DEFAULT_ADDRESSES.iter().len() {
        insert_account_info(&account_info, DEFAULT_ADDRESSES[i].clone(), DEFAULT_BALANCE).unwrap();
    }
    insert_meta_data(&protocol_inputs, 10, 100).expect("failed to updated metadata");

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

#[test]
fn init_db() {
    run(&TestInitDBOpts {
        dbpath: ("././bonsaidb").to_string(),
        force: Some(true),
        default_balance: Some(DEFAULT_BALANCE),
    })
    .unwrap()
}
