use std::net::SocketAddr;

use anyhow::Result;
use bitmask_enum::bitmask;
use jsonrpsee::{
    core::async_trait,
    proc_macros::rpc,
    server::{ServerBuilder, ServerHandle},
};
use service_config::ServiceConfig;

pub struct InternalRpcServer;
impl InternalRpcServer {
    /// Starts the RPC server which listens for internal calls.
    pub async fn start(service_config: ServiceConfig) -> Result<(ServerHandle, SocketAddr)> {
        let server = ServerBuilder::default()
            .build(format!(
                "{}:{}",
                service_config.rpc_address, service_config.rpc_port
            ))
            .await?;

        let addr = server.local_addr()?;
        let handle = server.start(service_config.into_rpc())?;

        Ok((handle, addr))
    }
}

#[rpc(server, client, namespace = "common")]
pub trait InternalRpcApi {
    #[method(name = "getStatus")]
    async fn status(&self) -> Result<(), jsonrpsee::core::Error>;
}

#[async_trait]
impl InternalRpcApiServer for ServiceConfig {
    async fn status(&self) -> Result<(), jsonrpsee::core::Error> {
        Ok(())
    }
}

/// An enum representing the service type. Compute, Storage, for example. More to come in the future.
enum ServiceType {
    /// A service that will accept (and execute) compute jobs
    Compute,
    /// A service that will handle web3 content-addressed persistence of binary blobs
    Storage,
    /// A service that supports the Versatus blockchain protocol(s)
    Blockchain,
}

/// A bitmask of capabilities supported by a particular service.
/// Subject to change, batteries not included.
#[bitmask]
enum ServiceCapabilities {
    /// This compute service supports execution of WASM/WASI
    Wasi,
    /// This compute service supports execution of X86_64 code
    Amd64,
    /// This compute service supports execution of ARM64 code
    Aarch64,
    /// This compute service supports execution of RISC-V code
    Riscv,
    /// This compute service supports consensus (smart contract) jobs
    Consensus,
    /// This compute service supports Function-as-a-Service (FaaS) jobs
    Faas,
    /// This compute service supports long-running Node-JS jobs
    NodeJs,
    /// This storage service supports the IPFS web3 storage protocol
    Ipfs,
    /// This storage service's data store is on resilient storage
    Resilient,
}

/// A version number
struct VersionNumber {
    major: u8,
    minor: u8,
    patch: u8,
}

struct ServiceStatusResponse {
    /// Type of service (see above)
    service_type: ServiceType,
    /// Capabilities of this service (see above)
    service_capabilities: ServiceCapabilities,
    /// A string naming the implementation of this storage service (future proofing).
    service_implementation: String,
    /// The version number of this service
    service_version: VersionNumber,
    /// The current uptime (seconds.ns) of the service
    service_uptime: std::time::Duration,
}
