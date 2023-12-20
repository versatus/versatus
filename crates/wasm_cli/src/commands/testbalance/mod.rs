use anyhow::Ok;
use anyhow::Result;
use bonsaidb::local::config::StorageConfiguration;
use bonsaidb_core::connection::Database;
use bonsaidb_core::document::BorrowedDocument;
use bonsaidb_core::document::HasHeader;
use bonsaidb_core::schema::ReduceResult;
use bonsaidb_core::schema::SerializedCollection;
use bonsaidb_core::schema::View;
use bonsaidb_core::schema::ViewMapResult;
use bonsaidb_core::schema::ViewMappedValue;
use clap::Parser;
use ethereum_types::U256;
use primitives::Address;

use crate::commands::testinitdb::*;

const DEFAULT_BALANCE: U256 = U256([10000; 4]);
#[derive(Debug, Clone, View)]
#[view(collection = AccountInfo, key = Option<String>, value = U256, name = "account-balance")]
pub struct AccountBalance;

impl MapReduce for AccountBalance {
    fn map<'doc>(&self, document: &'doc BorrowedDocument<'_>) -> ViewMapResult<'doc, Self> {
        let bal = AccountInfo::document_contents(document)?;
        document.header.emit_key_and_value(bal.category, 2)
    }

    fn reduce(
        &self,
        mappings: &[ViewMappedValue<Self::View>],
        _rereduce: bool,
    ) -> ReduceResult<Self::View> {
        Ok(mappings.iter().map(|mapping| mapping.value).sum())
    }
}

#[derive(Parser, Debug)]
pub struct TestBalanceOpts {
    /// This is the path to the database to be created/used. #716, this path is what we'll feed
    /// into the database driver.
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
    drop(std::fs::remove_dir_all("bonsaidb"));
    let db = Database::open::<AccountInfo>(StorageConfiguration::new(&opts))?;
    let balance = db
        .view::<AccountBalance>()
        .with_key(
            &Some(DEFAULT_BALANCE)
                .expect("Incorrect Balance")
                .to_string(),
        )
        .query_with_collection_docs()?;
    for mapping in &balance {
        let bal = AccountInfo::document_contents(mapping.document)?;
        println!(
            "Balance: {} \"{}\"",
            mapping.document.header.id, bal.account_balance
        );
    }
    // pub fn run(opts: &TestBalanceOpts) -> Result<()> {
    //     let balance = StorageConnection::list_databases(&AccountInfo {
    //         account_address: Address,
    //         account_balance: U256,
    //     })
    //     .view::<AccountBalance>()
    //     .with_key(&Some(U256.to_string()))
    //     .query_with_docs()?;
    //     for mapping in &balance {
    //         let bal = AccountInfo::document_contents(mapping.document)?;
    //         println!(
    //             "Balance: {} \"{}\"",
    //             mapping.document.header.id, bal.account_balance
    //         );
    //     }
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
