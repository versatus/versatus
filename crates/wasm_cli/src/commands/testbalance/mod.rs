use anyhow::Result;
use bonsaidb::{
    core::{connection::StorageConnection, schema::SerializedCollection},
    local::{
        config::{Builder, StorageConfiguration},
        Storage,
    },
};
use clap::Parser;
use ethnum::U256;
use primitives::Address;

use crate::commands::testinitdb::*;

#[derive(Parser, Debug, Clone)]
pub struct TestBalanceOpts {
    /// This is the path to the database to be created/used. #716, this path is what we'll feed
    /// into the database driver.
    // TODO: Make this an option, and look for the db path in the file tree
    // using std::fs or std::env if we choose to allow it.
    // REASON: If the db path is the current directory, we should infer this.
    // Otherwise, we should look for the db path in parent directories only
    // erring when one isn't found.
    #[clap(short, long)]
    pub dbpath: String,
    /// The address of the account to check the balance of.
    #[clap(short, long)]
    pub address: Address,
    /// Balance value we expect.
    #[clap(short, long)]
    pub balance: U256,
}

/// Checks the balance of an address matches the value provided and returns Ok/0 to the operating
/// system if it does, otherwise returns Err/1 to the operating system if they don't match.
pub fn run(opts: &TestBalanceOpts) -> Result<()> {
    let storage_connection =
        Storage::open(StorageConfiguration::new(&opts.dbpath).with_schema::<AccountBalance>()?)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to retrieve database at path '{}': {e:?}",
                    &opts.dbpath
                )
            })?;
    let db = storage_connection.database::<AccountBalance>("account-balance")?;

    let key = AccountAddress {
        address: opts.address.0,
    };
    let retrieved = AccountBalance::get(&key, &db)
        .map_err(|e| anyhow::anyhow!("failed to open document: {e:?}"))?
        .ok_or(anyhow::anyhow!(
            "failed to retrieve account balance for account address '{:?}' at database path '{}'",
            &opts.address,
            &opts.dbpath
        ))?;

    // TODO: Make this more robust!
    assert_eq!(opts.balance, retrieved.contents.value);

    Ok(())
}

#[test]
fn test_bal() {
    run(&TestBalanceOpts {
        dbpath: (DEFAULT_DB_PATH).to_string(),
        address: DEFAULT_ADDRESSES[7].clone(),
        balance: DEFAULT_BALANCE,
    })
    .unwrap()
}
