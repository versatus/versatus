//! This module defines the ComputeRuntime trait adhered to by Versatus compute runtimes.
use crate::{oci_runc::OpenComputeRuntime, oci_wasm::OciWasmRuntime};
use anyhow::{anyhow, Context, Result};
use bitmask_enum::bitmask;
use flate2::write::GzEncoder;
use flate2::Compression;
use internal_rpc::job_queue::job::ComputeJobExecutionType;
use internal_rpc::{api::IPFSDataType, api::InternalRpcApiClient, client::InternalRpcClient};
use log::{debug, info};
use mktemp::Temp;
use serde_derive::{Deserialize, Serialize};
use service_config::ServiceConfig;
use std::collections::HashMap;
use std::env;
use std::fs::{create_dir, hard_link, metadata, set_permissions, File};
use std::io::{BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use tar::Builder;
use telemetry::request_stats::RequestStats;
use web3_pkg::web3_pkg::{Web3ObjectType, Web3Package};

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
        Ok(serde_json::from_reader(reader)
            .map_err(|e| anyhow!("failed to parse CidManifest from manifest file: {e:?}"))?)
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
        inputs: &str,
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
        //let runtime_root = "/tmp/runtime";
        info!("Runtime root for {} is {}", job_id, runtime_root);

        // Read the CID manifest file. This makes a runtime-configurable map between CIDs of web3
        // packages of compute runtimes that we trust on our network and the job types they're to
        // be used for. Control over this file as it's used and deployed across our network is a
        // key part of the security of our whole supply chain. Ease of supportability dictates that
        // we make this runtime-configurable, but we should move toward taking steps to ensure that
        // this is updated safely across the network and resistent to supply-chain attacks.
        //
        // At a point in the future, this data will likely be written to and retrieved from the
        // blockchain, giving us a network-wide standard version for each package, whilst still
        // allowing it to be overridden locally when testing/developing new runtime stacks.

        let manifest_file: String = match env::var("COMPUTE_MANIFEST") {
            Ok(var) => var,
            Err(_) => "/opt/versatus/etc/manifest.json".to_string(),
        };
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
        // We currently wrap this in a tokio runtime to keep it sync, but there's no reason we
        // can't make this whole module async when calling from elsewhere.
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async { Self::retrieve_package(runtime_root, storage, package_cid).await })?;
        stats.stop("payload".to_string())?;

        // Retrieve runtime package by CID
        stats.start("runtime".to_string())?;
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            Self::retrieve_package(runtime_root, storage, &manifest.entries[&job_type]).await
        })?;
        stats.stop("runtime".to_string())?;

        // Execute job
        stats.start("execute".to_string())?;
        let job_set: JobSet = JobSet {
            job_id: job_id.to_string(),
            payload_id: package_cid.to_string(),
            runtime_id: manifest.entries[&job_type].to_string(),
        };
        let mut ret = String::new();
        match job_type {
            ComputeJobExecutionType::SmartContract => {
                // The Kontain unikernel crashes with under certain workloads. Need to work out why
                // before making it the default again.
                //let r = KontainWasmRuntime {};
                let r = OciWasmRuntime {};
                ret = r.execute(&job_set, runtime_root, inputs)?;
                debug!("Job ID {} returned {}", job_set.job_id, ret);
            }
            ComputeJobExecutionType::AdHoc => {
                let r = OpenComputeRuntime {};
                ret = r.execute(&job_set, runtime_root, inputs)?;
                debug!("Job ID {} returned {}", job_set.job_id, ret);
            }
            ComputeJobExecutionType::Null => {} //_ => return Err(anyhow!("Unsupported compute job type {:?}", job_type)),
        }
        stats.stop("execute".to_string())?;

        // Perform post-execution tasks
        stats.start("post-exec".to_string())?;
        // We probably want to catch failures from r.execute() above and still call post_execute()
        // here, because that'll cause us to grab a tarball.
        Self::post_execute(job_id, runtime_root)?;
        stats.stop("post-exec".to_string())?;
        // Ret contains job stdout
        Ok(ret)
    }

    /// Retrieve a package from the web3 blob store via an internal RPC.
    async fn retrieve_package(
        runtime_root: &str,
        config: &ServiceConfig,
        package_cid: &str,
    ) -> Result<()> {
        let client = InternalRpcClient::new(config.rpc_socket_addr()?).await?;
        info!("Retrieving CID {}", package_cid);
        let res = client.0.get_data(package_cid, IPFSDataType::Dag).await?;

        let package_dir = format!("{}/{}", &runtime_root, &package_cid);
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
            let obj_file = format!("{}/{}", &package_dir, &obj.object_cid.cid);
            let mut f = File::create(&obj_file).context(format!(
                "Creating file {}/{}",
                &package_dir, &obj.object_cid.cid
            ))?;
            f.write_all(&data)?;

            // Use the role annotation to create a hard link that symbolises which binary this
            // object is. For example, a smart contract runtime package will contain three
            // executables with the roles krun, km and versatus-wasm. This just helps the runtimes
            // to find the specific binaries they're looking for regardless of content.
            if obj.object_annotations.contains_key("role") {
                info!("Object role is {}", &obj.object_annotations["role"]);
                let src = format!("{}/{}", &package_dir, &obj.object_cid.cid);
                let dest = format!("{}/{}", &package_dir, &obj.object_annotations["role"]);
                hard_link(&src, &dest)?;

                // Make it executable if it needs to be.
                if obj.object_type == Web3ObjectType::Executable {
                    info!("Making {} executable", dest);
                    let mut perms = metadata(&dest)?.permissions();
                    perms.set_mode(0o755);
                    set_permissions(&dest, perms)?;
                }
            }
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

        info!("Created diagnostics bundle of files in {}", tarball_path);
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

/// A set of components representing a compute job. Primarily a user-defined payload such as a
/// smart contract, and the runtime components needed to execute the job.
pub struct JobSet {
    /// The unique ID of this job instance.
    pub job_id: String,
    /// A string representing the payload ID (usually a CID) as used on the filesystem to point to
    /// developer-payload components such as contracts.
    pub payload_id: String,
    /// A string representing the runtime ID (usually a CID) as used on the filesystem to point to
    /// the runtime components.
    pub runtime_id: String,
}

/// Common functionality that a compute runtime must expose.
pub trait ComputeRuntime {
    fn capabilities() -> ComputeRuntimeCapabilities;
    fn domainname() -> &'static str;
    fn execute(&self, job_set: &JobSet, runtime_path: &str, inputs: &str) -> Result<String>;
}
