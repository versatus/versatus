use crate::commands::publish::VERSATUS_STORAGE_ADDRESS;
use anyhow::Result;
use clap::Parser;
use std::net::{IpAddr, ToSocketAddrs};
use std::str::from_utf8;
use web3_pkg::web3_pkg::Web3Package;
use web3_pkg::web3_store::Web3Store;

#[derive(Parser, Debug)]
pub struct FetchMetadataOpts {
    /// The storage server address
    #[clap(short, long, value_parser, value_name = "STORAGE_SERVER")]
    pub storage_server: Option<String>,

    /// The path to the WASM object file to load and describe
    #[clap(short, long, value_parser, value_name = "CID")]
    pub cid: String,

    #[clap(short, long, value_parser, value_name = "IS_SRV_RECORD")]
    pub is_srv: bool,

    /// Flag that indicates whether storage server is running locally
    #[clap(short, long, value_parser, value_name = "LOCAL")]
    pub is_local: bool,
}

/// Fetch metadata of web3 package from the network.
pub fn run(opts: &FetchMetadataOpts) -> Result<()> {
    let store = if let Some(address) = opts.storage_server.as_ref() {
        if let Ok(ip) = address.parse::<IpAddr>() {
            Web3Store::from_multiaddr(ip.to_string().as_str())?;
        } else if address.to_socket_addrs().is_ok() {
            Web3Store::from_hostname(address, opts.is_srv)?;
        } else {
            return Err(anyhow::Error::msg(
                "Address is neither hostname nor IP address",
            ));
        }
        Web3Store::local()?
    } else if opts.is_local {
        Web3Store::local()?
    } else {
        Web3Store::from_hostname(VERSATUS_STORAGE_ADDRESS, opts.is_srv)?
    };

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let obj_result = store.read_dag(opts.cid.as_str()).await;
        let obj = match obj_result {
            Ok(obj) => obj,
            Err(err) => {
                eprintln!("Error reading DAG: {}", err);
                return;
            }
        };

        let json_result = from_utf8(&obj);
        let json = match json_result {
            Ok(json) => json,
            Err(err) => {
                eprintln!("Error converting dag metadata to UTF-8: {}", err);
                return;
            }
        };

        let pkg_result: Result<Web3Package, _> = serde_json::from_str(json);
        let pkg = match pkg_result {
            Ok(pkg) => pkg,
            Err(err) => {
                eprintln!("Error deserializing JSON: {}", err);
                return;
            }
        };
        println!("{}", pkg);
    });

    Ok(())
}
