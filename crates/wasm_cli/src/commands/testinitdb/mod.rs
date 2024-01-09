use anyhow::Result;
use bonsaidb::{
    core::{
        connection::{Connection, StorageConnection},
        document::{CollectionDocument, Emit},
        key::Key,
        schema::{
            Collection, CollectionMapReduce, ReduceResult, SerializedCollection, View,
            ViewMapResult, ViewMappedValue, ViewSchema,
        },
    },
    local::{
        config::{Builder, StorageConfiguration},
        Database, Storage,
    },
};
use clap::Parser;
use ethnum::U256;
use primitives::Address;
use serde::{Deserialize, Serialize};

#[cfg(test)]
pub const DEFAULT_DB_PATH: &str = "./bonsaidb";
pub const DEFAULT_VERSION: i32 = 1;
pub const DEFAULT_BLOCK_HEIGHT: u64 = 10;
pub const DEFAULT_BLOCK_TIME: u64 = 1704018000;
pub const DEFAULT_BALANCE: U256 = U256([10000; 2]);
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
    pub force: bool,
    /// Default balance for new test accounts created. The protocol supports values up to
    /// [ethnum::U256] in size, but u128 ought to be fine for now.
    #[clap(short, long)]
    pub default_balance: Option<U256>,
    #[clap(short, long)]
    pub address: Option<Address>,
}

const ACCOUNT_BALANCE_NAME: &str = "account-balance";
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

#[derive(Debug, Clone, View, ViewSchema)]
#[view(collection = ProtocolInputs, key = i32, value = (u64, u64), name = "by-version")]
pub struct ProtocolView;
impl CollectionMapReduce for ProtocolView {
    fn map<'doc>(
        &self,
        document: CollectionDocument<ProtocolInputs>,
    ) -> ViewMapResult<'doc, Self::View> {
        document.header.emit_key_and_value(
            document.contents.version,
            (document.contents.block_height, document.contents.block_time),
        )
    }

    fn reduce(
        &self,
        mappings: &[ViewMappedValue<'_, Self>],
        _rereduce: bool,
    ) -> ReduceResult<Self::View> {
        let mut latest_version = mappings[0].key;
        let mut block_height_and_time: (u64, u64) = mappings[0].value;
        for mapping in mappings.iter() {
            if mapping.key > latest_version {
                latest_version = mapping.key;
                block_height_and_time = mapping.value;
            }
        }
        Ok(block_height_and_time)
    }
}

const PROTOCOL_INPUTS_NAME: &str = "protocol-inputs";
#[derive(Collection, Serialize, Deserialize, Clone, Parser, Debug)]
#[collection(name = "protocol-inputs", views = [ProtocolView])]
pub struct ProtocolInputs {
    pub version: i32,
    /// The block number/height of the block currently being processed
    pub block_height: u64,
    /// The timestamp of the block currently being processed
    pub block_time: u64,
}

pub(crate) fn open_storage(path: &String) -> Result<Storage> {
    Ok(Storage::open(
        StorageConfiguration::new(path)
            .with_schema::<AccountBalance>()?
            .with_schema::<ProtocolInputs>()?,
    )?)
}

// TODO: Make this an associated function on ProtocolInputs
fn insert_protocol_inputs<C: Connection>(
    connection: &C,
    version: i32,
    block_height: u64,
    block_time: u64,
) -> Result<(), bonsaidb::core::Error> {
    ProtocolInputs {
        version,
        block_height,
        block_time,
    }
    .push_into(connection)?;
    Ok(())
}

fn insert_test_balances(account_connection: &Database) -> Result<()> {
    for address in DEFAULT_ADDRESSES.iter() {
        let key = AccountAddress { address: address.0 };
        AccountBalance {
            value: DEFAULT_BALANCE,
        }
        .insert_into(&key, account_connection)?;
    }
    Ok(())
}

fn insert_balance_at_address(opts: &TestInitDBOpts, account_connection: &Database) -> Result<()> {
    if let Some(address) = &opts.address {
        let key = AccountAddress { address: address.0 };
        let value = if let Some(balance) = opts.default_balance {
            balance
        } else {
            println!(
                "Default balance is None. Initializing account for address '{:?}' with value 0u256",
                &opts.address
            );
            Default::default()
        };
        AccountBalance { value }.insert_into(&key, account_connection)?;
    } else if opts.default_balance.is_some() {
        println!(
            "A default balance was given without being assigned an address.
No account was created. Please provide an address and try again."
        );
    }
    Ok(())
}

/// Initialises a new database for keeping standalone state typically provided by a blockchain.
/// This allows some standalone testing of smart contracts without needing access to a testnet and
/// can also potentially be integrated into common CI/CD frameworks.
pub fn run(opts: &TestInitDBOpts) -> Result<()> {
    let storage_connection = if opts.force {
        drop(std::fs::remove_dir_all(&opts.dbpath));
        open_storage(&opts.dbpath)?
    } else {
        open_storage(&opts.dbpath).map_err(|e| {
            anyhow::anyhow!(
                "Failed to create new database at path '{}'.
Use `--force` to overwrite the database at the existing path.
FAIL: {e:?}",
                &opts.dbpath
            )
        })?
    };

    // Establish database connections
    let protocol_connection =
        storage_connection.create_database::<ProtocolInputs>(PROTOCOL_INPUTS_NAME, true)?;
    let account_connection =
        storage_connection.create_database::<AccountBalance>(ACCOUNT_BALANCE_NAME, true)?;

    insert_protocol_inputs(
        &protocol_connection,
        DEFAULT_VERSION,
        DEFAULT_BLOCK_HEIGHT,
        DEFAULT_BLOCK_TIME,
    )?;

    insert_test_balances(&account_connection)?;

    insert_balance_at_address(opts, &account_connection)?;

    Ok(())
}

#[test]
fn init_db() {
    run(&TestInitDBOpts {
        dbpath: (DEFAULT_DB_PATH).to_string(),
        force: true,
        default_balance: Some(DEFAULT_BALANCE),
        address: None,
    })
    .unwrap()
}
