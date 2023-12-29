use anyhow::Result;
use bonsaidb::core::connection::Connection;
use bonsaidb::core::key::Key;
use bonsaidb::core::schema::{Collection, SerializedCollection};
use bonsaidb::local::config::{Builder, StorageConfiguration};
use bonsaidb::local::Storage;
use bonsaidb_core::connection::StorageConnection;
use clap::Parser;
use ethereum_types::U256;
use primitives::Address;
use serde::{Deserialize, Serialize};

#[cfg(test)]
pub const DEFAULT_DB_PATH: &str = "./bonsaidb";
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

const PROTOCOL_INPUTS_NAME: &str = "protocol-inputs";
#[derive(Collection, Serialize, Deserialize, Clone, Parser, Debug)]
#[collection(name = "protocol-inputs")]
pub struct ProtocolInputs {
    /// The block number/height of the block currently being processed
    // TODO: figure out if native ids will be useful
    // #[native_id]
    pub block_height: u64,
    /// The timestamp of the block currently being processed
    pub block_time: u64,
}

fn insert_protocol_inputs<C: Connection>(
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
    let storage_connection = if opts.force {
        drop(std::fs::remove_dir_all(&opts.dbpath));
        Storage::open(
            StorageConfiguration::new(&opts.dbpath)
                .with_schema::<AccountBalance>()?
                .with_schema::<ProtocolInputs>()?,
        )?
    } else {
        Storage::open(
            StorageConfiguration::new(&opts.dbpath)
                .with_schema::<AccountBalance>()?
                .with_schema::<ProtocolInputs>()?,
        )
        .map_err(|e| {
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

    // TODO: Create consts to replace these magic numbers
    //                                           vvvvvvv
    insert_protocol_inputs(&protocol_connection, 10, 100)?;

    // TODO: Abstract this into its own function, eg `fn insert_test_balances`
    // Insert default test address bytes
    for address in DEFAULT_ADDRESSES.iter() {
        let key = AccountAddress { address: address.0 };
        AccountBalance {
            value: DEFAULT_BALANCE,
        }
        .insert_into(&key, &account_connection)?;
    }

    // TODO: Abstract this into its own function, eg `fn insert_balance_at_address`
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
        AccountBalance { value }.insert_into(&key, &account_connection)?;
    } else if opts.default_balance.is_some() {
        println!(
            "A default balance was given without being assigned an address.
No account was created. Please provide an address and try again."
        );
    }

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
