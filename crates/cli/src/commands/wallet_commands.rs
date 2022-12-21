use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

use clap::{Parser, Subcommand};
use vrrb_core::event_router::Event;

use crate::result::{CliError, Result};

#[derive(clap::Parser, Debug)]
pub enum WalletConfigOpts {
    
}

#[derive(Debug, Subcommand)]
pub enum WalletCmd {
    /// creates a new wallet
    NewWallet,
    /// creates addresses for given wallet; input is number to be generated
    AddAddresses(usize),
    /// gets addresses for given wallet 
    // do we want to limit this to their own wallet addr?
    GetAddresses(String), 
    /// add tokens to a given address with balance
    AddTokens(String, HashMap<String, u8>),
    /// update token balances
    UpdateTokenBal(String, HashMap<String, u8>), 
    /// get token balances
    GetTokenBal(String, Vec<String>),
}

#[derive(Parser, Debug)]
pub struct WalletOpts {
    #[clap(subcommand)]
    pub subcommand: WalletCmd,
}

pub async fn newWallet(args: WalletOpts) -> Result<()> {
    let sub_cmd = args.subcommand;
    dbg!("in match subcommand");
    match sub_cmd {
        
        WalletCmd::Run(opts) => run(opts).await,
        _ => Err(CliError::InvalidCommand(format!("{:?}", sub_cmd))),
    }
} 

pub async fn exec(args: WalletOpts) -> Result<()> {
    let sub_cmd = args.subcommand;

    match sub_cmd {
        WalletCmd::Run(opts) => run(opts).await,
        _ => Err(CliError::InvalidCommand(format!("{:?}", sub_cmd))),
    }
}

