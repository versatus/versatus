use crate::commands::publish::VERSATUS_STORAGE_ADDRESS;
use anyhow::Result;
use clap::Parser;
use multiaddr::Multiaddr;
use std::net::{AddrParseError, SocketAddr};
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

    #[clap(
        short,
        long,
        value_name = "IS_SRV_RECORD",
        requires_if("STORAGE_SERVER", "IS_SRV_RECORD")
    )]
    pub is_srv: Option<bool>,

    /// Flag that indicates whether storage server is running locally
    #[clap(short, long, value_parser, value_name = "LOCAL")]
    pub is_local: bool,
}

impl FetchMetadataOpts {
    pub fn validate(&self) -> Result<()> {
        if self.storage_server.is_some() && self.is_srv.is_none() {
            return Err(anyhow::anyhow!(
                "If storage-server is provided, is_srv must also be provided."
            ));
        }
        Ok(())
    }
}
/// Fetch metadata of web3 package from the network.
pub fn run(opts: &FetchMetadataOpts) -> Result<()> {
    let is_srv = if let Some(value) = opts.is_srv {
        value
    } else {
        false
    };
    let store = if let Some(address) = opts.storage_server.as_ref() {
        if let Ok(ip) = address.parse::<Multiaddr>() {
            Web3Store::from_multiaddr(ip.to_string().as_str())?
        } else {
            Web3Store::from_hostname(address, is_srv)?
        }
    } else if opts.is_local {
        Web3Store::local()?
    } else if let Ok(addr) = std::env::var("VIPFS_ADDRESS") {
        let socket_addr: Result<SocketAddr, AddrParseError> = addr.parse();
        if let Ok(qualified_addr) = socket_addr {
            let (ip_protocol, ip) = match qualified_addr.ip() {
                std::net::IpAddr::V4(ip) => ("ip4".to_string(), ip.to_string()),
                std::net::IpAddr::V6(ip) => ("ip6".to_string(), ip.to_string()),
            };
            let port = qualified_addr.port().to_string();

            let multiaddr_string = format!("/{ip_protocol}/{ip}/tcp/{port}");

            Web3Store::from_multiaddr(&multiaddr_string)?
        } else {
            Web3Store::from_hostname(&addr, true)?
        }
    } else {
        Web3Store::from_hostname(VERSATUS_STORAGE_ADDRESS, true)?
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
