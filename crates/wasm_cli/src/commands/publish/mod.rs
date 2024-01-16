use anyhow::Result;
use clap::Parser;
use std::ffi::OsStr;
use std::net::{IpAddr, ToSocketAddrs};
use std::path::Path;
use std::path::PathBuf;
use web3_pkg::web3_pkg::{
    Web3ContentId, Web3ObjectType, Web3PackageArchitecture, Web3PackageBuilder, Web3PackageObject,
    Web3PackageObjectBuilder, Web3PackageType,
};
use web3_pkg::web3_store::Web3Store;

pub const VERSATUS_STORAGE_ADDRESS: &str = "storage.versatus.net";
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

    #[clap(short, long, value_parser, value_name = "IS_SRV_RECORD")]
    pub is_srv: bool,

    #[clap(short, long, value_parser, value_name = "RECURSIVE_PUBLISH")]
    pub recursive: bool,

    #[clap(short, long, value_parser, value_name = "LOCAL")]
    pub is_local: bool,
}

/// Generate a web3-native package from a smart contract and publish it to the network. This is a
/// stripped-down implementation of what's in the web3-pkg example that's supposed to be pretty
/// trivial for publishing a smart contract.
pub async fn run(opts: &PublishOpts) -> Result<()> {
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

    let mut objects: Vec<Web3PackageObject> = vec![];

    let cid = store.write_object(std::fs::read(&opts.wasm)?).await?;

    // Keep the filename portion of the path as a user-readable label
    let path = Path::new(&opts.wasm)
        .file_name()
        .unwrap_or(OsStr::new("unknown"))
        .to_str()
        .unwrap_or_default();

    let obj = Web3PackageObjectBuilder::default()
        .object_arch(Web3PackageArchitecture::Wasm32Wasi)
        .object_path(path.to_string().to_owned())
        .object_cid(Web3ContentId { cid })
        .object_type(Web3ObjectType::Executable)
        .build()?;

    objects.push(obj);

    let pkg_meta = Web3PackageBuilder::default()
        .pkg_version(opts.version)
        .pkg_name(opts.name.to_owned())
        .pkg_author(opts.author.to_owned())
        .pkg_type(Web3PackageType::SmartContract)
        .pkg_objects(objects)
        .pkg_replaces(vec![])
        .build()?;
    let json = serde_json::to_string(&pkg_meta)?;

    let cid = store.write_dag(json.into()).await?;

    println!("Content ID for Web3 Package is {}", cid);

    Ok(())
}
