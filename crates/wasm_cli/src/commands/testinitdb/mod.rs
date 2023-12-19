use anyhow::Result;
use bonsaidb::local::Storage;
use bonsaidb::{
    core::schema::SerializedCollection,
    local::config::{Builder, StorageConfiguration},
};
use bonsaidb_core::connection::{Connection, StorageConnection};
use bonsaidb_core::schema::{Collection, Schema};
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

#[derive(Debug, Schema)]
#[schema(name = "db-schema", collections = [AccountInfo, ProtocolInputs ])]
struct DBSchema;

#[derive(Clone, Parser, Debug)]
pub struct TestInitDBOpts {
    #[clap(short, long)]
    pub dbpath: String,
    /// Force DB to be initialised, even if it already exists. #716 I think that if this option is
    /// missing or false, we should only recreate the database from defaults if it doesn't already
    /// exist. If it exists, we should exit with a failure and an error message indicating that the
    /// database already exists and to use --force.
    #[clap(short, long)]
    pub force: Option<bool>,
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

// Templates for inserting information via connection.
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
            .with_schema::<ProtocolInputs>()?
            .with_schema::<DBSchema>()?,
    )?;

    storage.create_database::<AccountInfo>("account-info", true)?;
    let account_info = storage.database::<AccountInfo>("account-info")?;
    storage.create_database::<ProtocolInputs>("protocol-inputs", true)?;
    let protocol_inputs = storage.database::<ProtocolInputs>("protocol-inputs")?;

    // Push mock information for AccountInfo and ProtocolInputs to be stored in db.
    for i in 0..DEFAULT_ADDRESSES.iter().len() {
        insert_account_info(&account_info, DEFAULT_ADDRESSES[i].clone(), DEFAULT_BALANCE).unwrap();
    }
    insert_meta_data(&protocol_inputs, 10, 100).expect("failed to updated metadata");
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
