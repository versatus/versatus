use std::net::SocketAddr;

use crate::api::{IPFSDataType, InternalRpcApiServer, RpcResult};
use jsonrpsee::core::__reexports::serde_json;
use jsonrpsee::{
    core::async_trait,
    server::{ServerBuilder, ServerHandle},
};
use platform::services::*;
use service_config::ServiceConfig;
use tokio::runtime::Runtime;
use web3_pkg::web3_pkg::Web3Package;
use web3_pkg::web3_store::Web3Store;

pub struct InternalRpcServer;
impl InternalRpcServer {
    /// Starts the RPC server which listens for internal calls.
    /// The server will continue to run until the handle is consumed.
    pub async fn start(
        service_config: &ServiceConfig,
        service_type: ServiceType,
    ) -> anyhow::Result<(ServerHandle, SocketAddr)> {
        let rpc = InternalRpc::new(service_type)?;
        let server = ServerBuilder::default()
            .build(format!(
                "{}:{}",
                service_config.rpc_address, service_config.rpc_port
            ))
            .await?;

        let addr = server.local_addr()?;
        let handle = server.start(rpc.into_rpc())?;

        Ok((handle, addr))
    }
}

/// Represents all information available to the server and client.
/// Calls to the [`InternalRpcApi`] rely on this structure.
struct InternalRpc {
    /// An enum representing the service type. Compute, Storage, for example. More to come in the future.
    pub(crate) service_type: ServiceType,
    /// The time of the creation of the `InternalRpc`, used to get the uptime of a service.
    pub(crate) service_start: std::time::Instant,
    /// A bitmask of capabilities supported by a particular service.
    /// Subject to change, batteries not included.
    pub(crate) service_capabilities: ServiceCapabilities,
    /// The `CARGO_PKG_VERSION` as specified by `std::env`.
    pub(crate) version: VersionNumber,
}

impl InternalRpc {
    pub fn new(service_type: ServiceType) -> anyhow::Result<Self> {
        let extra_service_capabilities = ServiceCapabilities::try_from(platform::uname()?)?;
        Ok(Self {
            service_type: service_type.clone(),
            service_start: std::time::Instant::now(),
            service_capabilities: match service_type {
                ServiceType::Compute => {
                    ServiceCapabilities::Wasi
                        | ServiceCapabilities::Consensus
                        | extra_service_capabilities
                }
                ServiceType::Storage => ServiceCapabilities::Ipfs | extra_service_capabilities,
                _ => extra_service_capabilities,
            },
            version: VersionNumber::cargo_pkg(),
        })
    }

    async fn retrieve_object(&self, cid: &str) -> RpcResult<Vec<(String, Vec<u8>)>> {
        let store = Web3Store::local()?;
        let obj = store.read_object(cid).await?;
        Ok(vec![(cid.to_string(), obj)])
    }
    async fn retrieve_dag(&self, cid: &str) -> RpcResult<Vec<(String, Vec<u8>)>> {
        let store = Web3Store::local()?;
        let obj = store.read_dag(cid).await?;
        let pkg: Web3Package = serde_json::from_slice(&obj)?;
        let mut objs = Vec::new();
        for obj in &pkg.pkg_objects {
            let blob = store.read_object(&obj.object_cid.cid).await?;
            objs.push((obj.object_cid.cid.clone(), blob))
        }
        Ok(objs)
    }

    async fn pin_object_ipfs(&self, cid: &str, recursive: bool) -> RpcResult<Vec<String>> {
        let store = Web3Store::local()?;
        let obj = store.pin_object(cid, recursive).await?;
        Ok(obj)
    }

    async fn is_pinned_obj(&self, cid: &str) -> RpcResult<bool> {
        let store = Web3Store::local()?;
        let is_pinned = store.is_pinned(cid).await?;
        Ok(is_pinned)
    }
}

#[async_trait]
impl InternalRpcApiServer for InternalRpc {
    async fn status(&self) -> RpcResult<ServiceStatusResponse> {
        Ok(ServiceStatusResponse::from(self))
    }

    async fn get_data(
        &self,
        cid: &str,
        data_type: IPFSDataType,
    ) -> RpcResult<Vec<(String, Vec<u8>)>> {
        let rt = Runtime::new()?;
        rt.block_on(async {
            return match data_type {
                IPFSDataType::Object => self.retrieve_object(cid).await,
                IPFSDataType::Dag => self.retrieve_dag(cid).await,
            };
        })
    }

    async fn pin_object(&self, cid: &str, recursive: bool) -> RpcResult<Vec<String>> {
        let rt = Runtime::new()?;
        rt.block_on(async { self.pin_object_ipfs(cid, recursive).await })
    }
    async fn is_pinned(&self, cid: &str) -> RpcResult<bool> {
        let rt = Runtime::new()?;
        rt.block_on(async { self.is_pinned_obj(cid).await })
    }
}

impl<'a> From<&'a InternalRpc> for ServiceStatusResponse {
    fn from(value: &'a InternalRpc) -> Self {
        Self {
            service_type: value.service_type.clone(),
            service_capabilities: value.service_capabilities,
            service_implementation: "".to_string(),
            service_version: value.version.clone(),
            service_uptime: value.service_start.elapsed().as_secs(),
        }
    }
}
