use anyhow::Result;
use bonsaidb::core::key::Key;
use bonsaidb::core::schema::{Collection, Schema, SerializedCollection};
use bonsaidb::local::config::{Builder, StorageConfiguration};
use bonsaidb::local::Database;
use clap::Parser;
use ethereum_types::U256;
use primitives::Address;
use serde::{Deserialize, Serialize};

pub const DEFAULT_BALANCE: U256 = U256([10000; 4]);
pub const DEFAULT_ADDRESSES: &[Address; 10] = &[
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
    #[clap(short, long)]
    pub address: Option<String>,
}

//Schema for AccountBalance
#[derive(Debug, Schema)]
#[schema(name = "primary-keys", collections = [AccountBalance])]
pub struct AccountSchema;

//Collection of account balances relative to key (address) inserted.
#[derive(Debug, Serialize, Deserialize, Collection, Eq, PartialEq)]
#[collection(name = "account-balance", primary_key = AccountAddress)]
pub struct AccountBalance {
    pub(crate) value: U256,
}

//Key used to pull account's relative balance.
#[derive(Key, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct AccountAddress {
    pub address: [u8; 20],
}

// // Keeping these below, as we will need a seperate database for metadata.

// #[derive(Collection, Serialize, Deserialize, Clone, Parser, Debug)]
// #[collection(name = "protocol-inputs")]
// pub struct ProtocolInputs {
//     /// The block number/height of the block currently being processed
//     pub block_height: u64,
//     /// The timestamp of the block currently being processed
//     pub block_time: u64,
// }

// // Template for inserting information via connection.
// fn insert_meta_data<C: Connection>(
//     connection: &C,
//     block_height: u64,
//     block_time: u64,
// ) -> Result<(), bonsaidb::core::Error> {
//     ProtocolInputs {
//         block_height,
//         block_time,
//     }
//     .push_into(connection)?;
//     Ok(())
// }

/// Initialises a new database for keeping standalone state typically provided by a blockchain.
/// This allows some standalone testing of smart contracts without needing access to a testnet and
/// can also potentially be integrated into common CI/CD frameworks.
pub fn run(opts: &TestInitDBOpts) -> Result<()> {
    let address_bytes = &opts.address.clone().unwrap().into_bytes()[..20];
    let mut address = [0; 20];
    address.copy_from_slice(&address_bytes);

    drop(std::fs::remove_dir_all(&opts.dbpath));
    let db = Database::open::<AccountSchema>(StorageConfiguration::new(&opts.dbpath))?;

    let key = AccountAddress { address };
    AccountBalance {
        value: DEFAULT_BALANCE,
    }
    .insert_into(&key, &db)?;
    Ok(())
}

#[test]
fn init_db() {
    run(&TestInitDBOpts {
        dbpath: ("./bonsaidb").to_string(),
        force: Some(true),
        default_balance: Some(DEFAULT_BALANCE),
        address: Some(DEFAULT_ADDRESSES[0].to_string()),
    })
    .unwrap()
}
