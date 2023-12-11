use anyhow::Result;
use clap::Parser;
use ethereum_types::U256;

#[derive(Parser, Debug)]
pub struct AssertBalanceOpts {
    /// This is the path to the database to be created/used. #716, this path is what we'll feed
    /// into the database driver.
    #[clap(short, long)]
    pub dbpath: String,
    /// The address of the account to check the balance of.
    #[clap(short, long)]
    pub address: String,
    /// Balance value we expect.
    #[clap(short, long)]
    pub balance: U256,
}

/// Checks the balance of an address matches the value provided and returns Ok/0 to the operating
/// system if it does, otherwise returns Err/1 to the operating system if they don't match.
pub fn run(opts: &AssertBalanceOpts) -> Result<()> {
    // #716 Here we should do a query for the provided address, and compare its balance with the
    // balance provided. If they match, we should return success. If they don't, we should return
    // failure. It may even be worth returning a different failure if the account doesn't exist.
    //
    // In the case of success, there should be no output to stdout. In the case of failure, a clear
    // message should be displayed on stderr.
    Ok(())
}
