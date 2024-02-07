use anyhow::Result;
use clap::Parser;
use multiaddr::Multiaddr;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::net::AddrParseError;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;
use web3_pkg::web3_pkg::{
    Web3ContentId, Web3ObjectType, Web3PackageArchitecture, Web3PackageBuilder, Web3PackageObject,
    Web3PackageObjectBuilder, Web3PackageType,
};
use web3_pkg::web3_store::Web3Store;
pub const VERSATUS_STORAGE_ADDRESS: &str = "_storage._tcp.incomplete.io";
#[derive(Parser, Debug)]
pub struct PublishOpts {
    /// The path to the WASM object file to package and publish
    #[clap(short, long, value_parser, value_name = "FILE")]
    pub wasm: PathBuf,
    /// The name of the package to create
    #[clap(short, long, value_parser, value_name = "NAME")]
    pub name: String,
    /// The author of the package
    #[clap(short, long, value_parser, value_name = "AUTHOR")]
    pub author: String,
    /// The version of the package
    #[clap(short, long, value_parser, value_name = "VERSION")]
    pub version: u32,

    /// The storage server address
    #[clap(short, long, value_parser, value_name = "STORAGE_SERVER")]
    pub storage_server: Option<String>,

    #[clap(short, long, value_name = "IS_SRV_RECORD")]
    pub is_srv: Option<bool>,

    #[clap(short, long, value_parser, value_name = "RECURSIVE_PUBLISH")]
    pub recursive: bool,

    #[clap(short, long, value_parser, value_name = "LOCAL")]
    pub is_local: bool,
}

impl PublishOpts {
    pub fn validate(&self) -> Result<()> {
        if self.storage_server.is_some() && self.is_srv.is_none() {
            return Err(anyhow::anyhow!(
                "If storage-server is provided, is_srv must also be provided."
            ));
        }
        Ok(())
    }
}

/// Generate a web3-native package from a smart contract and publish it to the network. This is a
/// stripped-down implementation of what's in the web3-pkg example that's supposed to be pretty
/// trivial for publishing a smart contract.
pub fn run(opts: &PublishOpts) -> Result<()> {
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
    } else {
        if let Ok(addr) = std::env::var("VIPFS_ADDRESS") {
            let socket_addr: Result<SocketAddr, AddrParseError>  = addr.parse();
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
        }
    };

    // Define some package and object annotations to include.
    let mut o_ann = HashMap::<String, String>::new();
    o_ann.insert("role".to_string(), "contract".to_string());
    let mut p_ann = HashMap::<String, String>::new();
    p_ann.insert("status".to_string(), "test".to_string());

    let mut objects: Vec<Web3PackageObject> = vec![];
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let result: Result<()> = (async {
            let cid =
                store
                    .write_object(std::fs::read(&opts.wasm).map_err(|e| {
                        anyhow::Error::msg(format!("Error reading Wasm file: {}", e))
                    })?)
                    .await
                    .map_err(|e| anyhow::Error::msg(format!("Error writing object: {}", e)))?;

            let path = Path::new(&opts.wasm)
                .file_name()
                .unwrap_or(OsStr::new("unknown"))
                .to_str()
                .unwrap_or_default();

            let obj = Web3PackageObjectBuilder::default()
                .object_arch(Web3PackageArchitecture::Wasm32Wasi)
                .object_path(path.to_string().to_owned())
                .object_cid(Web3ContentId { cid })
                .object_annotations(o_ann)
                .object_type(Web3ObjectType::Executable)
                .build()
                .map_err(|e| {
                    anyhow::Error::msg(format!("Error occurred while building the package :{}", e))
                })?;

            objects.push(obj);

            let pkg_meta = Web3PackageBuilder::default()
                .pkg_version(opts.version)
                .pkg_name(opts.name.to_owned())
                .pkg_author(opts.author.to_owned())
                .pkg_type(Web3PackageType::SmartContract)
                .pkg_objects(objects)
                .pkg_annotations(p_ann)
                .pkg_replaces(vec![])
                .build()
                .map_err(|e| anyhow::Error::msg(format!("Error building package: {}", e)))?;

            let json = serde_json::to_string(&pkg_meta).map_err(|e| {
                anyhow::Error::msg(format!("Error serializing package metadata to JSON: {}", e))
            })?;

            let cid = store
                .write_dag(json.into())
                .await
                .map_err(|e| anyhow::Error::msg(format!("Error writing DAG: {}", e)))?;

            println!("Content ID for Web3 Package is {}", cid);
            Ok(())
        })
        .await;
        if let Err(e) = result {
            eprintln!("Error: {}", e);
        }
    });

    Ok(())
}
