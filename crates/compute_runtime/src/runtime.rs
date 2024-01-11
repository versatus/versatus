//! This module defines the ComputeRuntime trait adhered to by Versatus compute runtimes.
use anyhow::{Context, Result};
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
use std::fs::{create_dir, File};
use std::io::{BufReader, Write};
use tar::Builder;
use telemetry::request_stats::RequestStats;
use web3_pkg::web3_pkg::Web3Package;

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
    ) -> Result<String> {
        // Create a stats object to track how long we take to perform certain phases of execution.
        let mut stats = RequestStats::new("ComputeJobRunner".to_string(), job_id.to_string())?;
        info!(
            "Executing compute job {} from package '{}' as a {} job.",
            &job_id, &package_cid, job_type
        );

        // start initial prep
        stats.start("setup".to_string())?;
        // Create a temporary directory tree that will be cleaned up (unlinked) when tmp goes out
        // of scope.
        let tmp = Temp::new_dir()?;
        let runtime_root = &tmp.to_string_lossy();
        info!("Runtime root for {} is {}", job_id, runtime_root);

        // - read CID manifest
        // TODO: hard-coded and shouldn't be.
        let manifest_file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/sample-configs/manifest.json"
        );
        info!("Reading CID manifest from {}", manifest_file);
        let manifest: CidManifest = CidManifest::from_file(manifest_file)?;
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
            // TODO: Bail here.
        }
        stats.stop("setup".to_string())?;

        // Retrieve payload package by CID. This is the developer's code to be executed (eg, a
        // smart contract).
        stats.start("payload".to_string())?;

        // We currently wrap this in a tokio runtime to keep it sync, but there's no reason we
        // can't make this whole module async when calling from elsewhere.
        let rt = tokio::runtime::Runtime::new()?;
        let _ = rt.block_on(async {
            Self::retrieve_package(runtime_root, storage, &package_cid).await
        })?;
        stats.stop("payload".to_string())?;

        // Retrieve runtime package by CID. This is the package that contains the binaries we need
        // in order to be able to execute the above user payload.
        stats.start("runtime".to_string())?;
        let _ = rt.block_on(async {
            Self::retrieve_package(runtime_root, storage, &manifest.entries[&job_type]).await
        })?;
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

        info!("Cleaning up {:?}", &tmp);

        Ok("{ \"this\": \"is a test signal\" }".to_string())
    }

    /// Retrieve a package from the web3 blob store via an internal RPC.
    async fn retrieve_package(
        root_path: &str,
        config: &ServiceConfig,
        package_cid: &str,
    ) -> Result<()> {
        let client = InternalRpcClient::new(config.rpc_socket_addr()?).await?;
        info!("Retrieving CID {}", package_cid);
        let res = client.0.get_data(package_cid, IPFSDataType::Dag).await?;

        let package_dir = format!("{}/{}", &root_path, &package_cid);
        create_dir(&package_dir).context(format!("Creating package directory {}", &package_dir))?;

        let pkg: Web3Package = serde_json::from_slice(&res)?;
        info!(
            "Package '{}' version {} from '{}' is type {}",
            &pkg.pkg_name, &pkg.pkg_version, &pkg.pkg_author, &pkg.pkg_type
        );
        let mut f = File::create(format!("{}/metadata.json", &package_dir))
            .context(format!("Creating {}/metadata.json", &package_dir))?;
        f.write_all(&res)?;

        for obj in pkg.pkg_objects.iter() {
            info!(
                "Package {} contains link to object {}, arch {}",
                &package_cid, &obj.object_cid.cid, &obj.object_arch
            );
            // TODO: We should probably check our current architecture against the architecture of
            // the object and not download it if it's not going to run here.
            // Also, we should parse the CID to see whether it's a DAG object or a blob object
            // instead of assuming it's a blob, although the latter will be the case for the
            // near term.
            let data = client
                .0
                .get_data(&obj.object_cid.cid, IPFSDataType::Object)
                .await?;
            // Write it the object to a file.
            let mut f = File::create(format!("{}/{}", &package_dir, &obj.object_cid.cid)).context(
                format!("Creating file {}/{}", &package_dir, &obj.object_cid.cid),
            )?;
            f.write_all(&data)?;
        }

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

        archive.append_dir_all(".", runtime_root)?;
        let _ = archive.finish();

        info!("Created diagnostics bundle of {}", tarball_path);
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
