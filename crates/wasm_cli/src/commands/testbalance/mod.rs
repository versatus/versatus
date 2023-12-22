use anyhow::Result;
use bonsaidb::core::schema::SerializedCollection;
use bonsaidb::local::config::{Builder, StorageConfiguration};
use bonsaidb::local::Database;
use bonsaidb_core::key::NextValueError;
use bonsaidb_core::permissions::bonsai::DatabaseAction;
use bonsaidb_core::schema::Collection;
use clap::Parser;
use ethereum_types::U256;
use primitives::Address;

use crate::commands::testinitdb::*;

// const DEFAULT_BALANCE: U256 = U256([10000; 4]);

#[derive(Parser, Debug)]
pub struct TestBalanceOpts {
    /// This is the path to the database to be created/used. #716, this path is what we'll feed
    /// into the database driver.
    #[clap(short, long)]
    pub dbpath: String,
    /// The address of the account to check the balance of.
    #[clap(short, long)]
    pub address: Option<String>,
    /// Balance value we expect.
    #[clap(short, long)]
    pub balance: U256,
}

/// Checks the balance of an address matches the value provided and returns Ok/0 to the operating
/// system if it does, otherwise returns Err/1 to the operating system if they don't match.
pub fn run(opts: &TestBalanceOpts) -> Result<()> {
    // store the open DB in a variable like `let db = DB::Open();`
    // you'll use the dbpath to open that db. Then you can use the
    // get method and give it the address to find. If it finds that address
    // you can assert whether the balance is the same.
    let address_bytes = &opts.address.clone().unwrap().into_bytes()[..20];
    let mut address = [0; 20];
    address.copy_from_slice(&address_bytes);

    let db = Database::open(opts.dbpath);

    // drop(std::fs::remove_dir_all(&opts.dbpath));
    // let db = Database::open::<AccountSchema>(StorageConfiguration::new(&opts.dbpath))?;

    let key = AccountAddress { address };
    let inserted = AccountBalance {
        value: DEFAULT_BALANCE,
    }
    .insert_into(&key, &db)?;
    let retrieved = AccountBalance::get(&key, &db)?.expect("document not found");
    assert_eq!(inserted, retrieved);

    assert!(matches!(
        AccountBalance {
            value: DEFAULT_BALANCE,
        }
        .push_into(&db)
        .unwrap_err()
        .error,
        bonsaidb::core::Error::DocumentPush(_, NextValueError::Unsupported)
    ));
    // #716 Here we should do a query for the provided address, and compare its balance with the
    // balance provided. If they match, we should return success. If they don't, we should return
    // failure. It may even be worth returning a different failure if the account doesn't exist.
    //
    // In the case of success, there should be no output to stdout. In the case of failure, a clear
    // message should be displayed on stderr.
    Ok(())
}

#[test]
fn test_bal() {
    run(&TestBalanceOpts {
        dbpath: ("././bonsaidb").to_string(),
        address: primitives::Address([7; 20]),
        balance: ethereum_types::U256([10000; 4]),
    })
    .unwrap()
}
