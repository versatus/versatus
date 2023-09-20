use clap::clap_derive::ArgEnum;
use derive_builder::Builder;
use serde_derive::{Deserialize, Serialize};

/// An enum representing different flavours of package payload. In some cases, a package might
/// contain a smart contract (or potentially multiple smart contracts), in other cases it could be
/// a FaaS function destined for The Edge or a long-running cloud service. The runtime binaries
/// themselves are also packaged and shipped in this fashion too and are represented here.
#[derive(Debug, Default, Clone, Serialize, Deserialize, ArgEnum)]
#[serde(rename_all = "camelCase")]
pub enum Web3PackageType {
    /// Default is no executable payload (ie, documentation)
    #[default]
    None,
    /// A package containing binaries to be used for creating a smart contract runtime
    SmartContractRuntime,
    /// A package containing a smart contract
    SmartContract,
}

/// An enum representing different architectures/platforms a compute workload could be targetted
/// to.
#[derive(Debug, Default, Serialize, Deserialize, Clone, ArgEnum)]
#[serde(rename_all = "camelCase")]
pub enum Web3PackageArchitecture {
    /// Default is no executable architecture (ie, documentation)
    #[default]
    None,
    /// x64_64-unknown-linux-gnu target architecture
    Amd64Linux,
    /// x64_64-unknown-linux-musl target architecture
    Amd64Musl,
    /// aarch64-unknown-linux-gnu target architecture
    Aarch64Linux,
    /// aarch64-unknown-linux-musl target architecture
    Aarch64Musl,
    /// wasm32-wasi target architecture
    Wasm32Wasi,
}

/// A struct representing a content ID. Currently somewhat specific to IPFS CIDs and IPLD's
/// DAG-JSON format. The member name 'cid' is renamed by serde to '/' specifically to appease the
/// DAG-JSON gods.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Web3ContentId {
    #[serde(rename = "/")]
    pub cid: String,
}

/// An enum representing the type of object within the package. This is only as accurate as the
/// package publisher makes it.
#[derive(Debug, Default, Serialize, Deserialize, Clone, ArgEnum)]
#[serde(rename_all = "camelCase")]
pub enum Web3ObjectType {
    #[default]
    None,
    /// The publisher is suggesting that this object is a document.
    Document,
    /// The publisher is suggesting that this object is an executable.
    Executable,
    /// The publisher is suggesting that this object is an image.
    Image,
    /// The publisher is suggesting that this object is malware.
    Malware,
}

/// A struct representing an object within a package -- usually a binary file that makes up a
/// compute job module.
#[derive(Default, Debug, Clone, Serialize, Deserialize, Builder)]
#[serde(rename_all = "camelCase")]
pub struct Web3PackageObject {
    /// The architecture that this object is targetted to for execution.
    pub object_arch: Web3PackageArchitecture,
    /// The path to the object on the local filesystem
    pub object_path: String,
    /// Annotation hinting at the type of object that this is.
    pub object_type: Web3ObjectType,
    /// The content ID of the object within IPFS
    pub object_cid: Web3ContentId,
}

/// A structure representing the metadata of a compute package. A compute package may contain one
/// or more objects (see [Web3PackageObject] above) that represent binaries compatible with the
/// compute stack.
#[derive(Default, Debug, Clone, Serialize, Deserialize, Builder)]
#[serde(rename_all = "camelCase")]
pub struct Web3Package {
    /// Package management format version. Set internally.
    #[builder(default = "2")]
    #[builder(private)]
    pub api_version: u32,
    /// Package version as specified by the package maintainer.
    pub pkg_version: u32,
    /// A string representing the package name.
    pub pkg_name: String,
    /// A string representing the package author
    pub pkg_author: String,
    /// An enum representing the type of payload in this package.
    pub pkg_type: Web3PackageType,
    /// A vector of objects that this package contains
    pub pkg_objects: Vec<Web3PackageObject>,
    /// A vector of packages that this replaces. XXX: This could be problematic when exporting a
    /// DAG when there's a long history.
    pub pkg_replaces: Vec<Web3ContentId>,
}
