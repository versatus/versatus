//! This module defines the ComputeRuntime trait adhered to by Versatus compute runtimes.
use anyhow::Result;
use bitmask_enum::bitmask;
use flate2::write::GzEncoder;
use flate2::Compression;
use internal_rpc::{api::IPFSDataType, api::InternalRpcApiClient, client::InternalRpcClient};
use log::info;
use mktemp::Temp;
use serde_derive::{Deserialize, Serialize};
use service_config::ServiceConfig;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind};
use tar::Builder;
use telemetry::request_stats::RequestStats;
use walkdir::WalkDir;

// Two invalid CIDs to use within testing. Shouldn't ever be used in the wild, but useful for
// testing along with the Null execution type below.
pub const NULL_CID_TRUE: &str = "null-cid-true";
pub const NULL_CID_FALSE: &str = "null-cid-false";

/// The type of job we're intending to execute.
#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ComputeJobExecutionType {
    /// A Smart Contract job requiring a runtime capable of assembling JSON input and executing
    /// WASM.
    SmartContract,
    /// An ad-hoc execution job. Always local to a node, and primarily used for
    /// testing/development.
    AdHoc,
    /// A null job type primarily used for internal/unit testing and runs nothing.
    Null,
}

impl fmt::Display for ComputeJobExecutionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Self::SmartContract => {
                write!(f, "Smart Contract")
            }
            Self::AdHoc => {
                write!(f, "Ad Hoc Task")
            }
            Self::Null => {
                write!(f, "Null Task")
            }
        }
    }
}
impl std::str::FromStr for ComputeJobExecutionType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let job = match s.trim().to_lowercase().as_str() {
            "contract" | "smart-contract" => ComputeJobExecutionType::SmartContract,
            "adhoc" | "ad-hoc" => ComputeJobExecutionType::AdHoc,
            "null" => ComputeJobExecutionType::Null,
            _ => {
                return Err(anyhow::anyhow!(
                    "failed to parse compute job type from string"
                ));
            }
        };
        Ok(job)
    }
}

/// A runtime-configurable mapping between [ComputeJobExecutionType]s and published package CIDs.
/// This will allow us to set a bunch of sane defaults that can be overridden as new packages
/// become available, or as we want to test them. Later, we'll likely want to add GPG signatures to
/// a lot of these to give operators confidence that they know who published the binaries.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CidManifest {
    /// A format version number
    pub version: u32,
    /// A map between execution type and CID
    pub entries: HashMap<ComputeJobExecutionType, String>,
}

impl CidManifest {
    pub fn from_file(path: &str) -> Result<CidManifest> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }
}

/// A struct representing the machine to run a compute job. This is a wrapper around all of the
/// [ComputeRuntime] implementations.
#[derive(Debug)]
pub struct ComputeJobRunner {}

impl ComputeJobRunner {
    pub fn run(
        job_id: &str,
        package_cid: &str,
        job_type: ComputeJobExecutionType,
        storage: &ServiceConfig,
    ) -> Result<()> {
        // Create a stats object to track how long we take to perform certain phases of execution.
        let mut stats = RequestStats::new("ComputeJobRunner".to_string(), job_id.to_string())?;
        info!(
            "Executing compute job {} from package '{}' as a {} job.",
            &job_id, &package_cid, job_type
        );

        // start initial prep
        stats.start("setup".to_string())?;
        // Create a temporary directory tree that will be cleaned up (unlinked) when tmp goes out
        // of scope. This is easy with the mktemp::Temp crate, but the hurdles that Rust makes us
        // jump through to get a string representation of the path out is insane. Yes, it is
        // possible for paths to contain non-UTF8 characters, but given that the whole path was
        // created from Rust, Rust could guarantee that this was in fact UTF8 from the start. No
        // need to map an OsString into an ambiguous error.
        //
        // Making matters more fun, we can't even barf with a reasonable error code.
        // ErrorKind::InvalidFilename (the most accurate description) is only available as a
        // feature in nightly builds. FML.
        let tmp = Temp::new_dir()?.to_path_buf();
        let runtime_root = &tmp
            .into_os_string()
            .into_string()
            .map_err(|_| Error::new(ErrorKind::NotFound, "Cannot parse file path"))?;
        info!("Runtime root for {} is {}", job_id, runtime_root);

        // - read CID manifest
        // TODO: hard-coded and shouldn't be.
        let manifest_file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/sample-configs/manifest.json"
        );
        info!("Reading CID manifest from {}", manifest_file);
        let manifest: CidManifest = CidManifest::from_file(&manifest_file)?;
        //dbg!(manifest);
        if manifest.entries.contains_key(&job_type) {
            info!(
                "Using runtime package with CID {}",
                manifest.entries[&job_type]
            );
        } else {
            info!(
                "No CID found for job type {:?} in {:?}",
                &job_type, &manifest.entries
            );
        }
        stats.stop("setup".to_string())?;

        // Retrieve payload package by CID
        stats.start("payload".to_string())?;
        // TODO: retrieve payload package by CID
        // We currently wrap this in a tokio runtime to keep it sync, but there's no reason we
        // can't make this whole module async when calling from elsewhere.
        let rt = tokio::runtime::Runtime::new()?;
        let _ = rt.block_on(async { Self::retrieve_package(&storage, &package_cid).await })?;
        stats.stop("payload".to_string())?;

        // Retrieve runtime package by CID
        stats.start("runtime".to_string())?;
        // TODO: retrieve runtime package by CID
        stats.stop("runtime".to_string())?;

        // Execute job
        stats.start("execute".to_string())?;

        match job_type {
            ComputeJobExecutionType::SmartContract => {}
            ComputeJobExecutionType::AdHoc => {}
            ComputeJobExecutionType::Null => {} //_ => return Err(anyhow!("Unsupported compute job type {:?}", job_type)),
        }
        stats.stop("execute".to_string())?;

        // Perform post-execution tasks
        stats.start("post-exec".to_string())?;
        Self::post_execute(job_id, runtime_root)?;
        stats.stop("post-exec".to_string())?;
        Ok(())
    }

    /// Retrieve a package from the web3 blob store via an internal RPC.
    async fn retrieve_package(config: &ServiceConfig, package_cid: &str) -> Result<()> {
        let client = InternalRpcClient::new(config.rpc_socket_addr()?).await?;
        info!("Retrieving CID {}", package_cid);
        let res = client.0.get_data(package_cid, IPFSDataType::Object).await?;
        dbg!(res);
        Ok(())
    }

    /// Perform post-execution tasks, such as collection of diagnostics.
    fn post_execute(job_id: &str, runtime_root: &str) -> Result<()> {
        // Create a tarball of of the container runtime and the logs for diagnostics. Once we
        // release to mainnet, we should look at making this optional and off by default, but for
        // now, we'll collect data on every job. TODO: Is $HOME the right place for this? It *is*
        // in production, but probably isn't for unit tests....
        let tarball_path = format!("{}/{}-diag.tar.gz", std::env!("HOME"), &job_id);
        let file = File::create(&tarball_path)?;
        let enc = GzEncoder::new(file, Compression::default());
        let mut archive = Builder::new(enc);

        let mut count = 0;
        for entry in WalkDir::new(&runtime_root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let filename = entry.path();
            archive.append_path(filename)?;
            count += 1;
        }

        info!(
            "Created diagnostics bundle of {} files in {}",
            count, tarball_path
        );
        Ok(())
    }
}

/// A bitmask of capabilities that a particular compute runtime has.
#[bitmask]
pub enum ComputeRuntimeCapabilities {
    Wasm,
    Native,
    Python,
}

/// Common functionality that a compute runtime must expose.
pub trait ComputeRuntime {
    fn capabilities() -> ComputeRuntimeCapabilities;
    fn domainname() -> &'static str;
    fn setup(&self, job_id: &str, runtime_path: &str) -> Result<()>;
}
