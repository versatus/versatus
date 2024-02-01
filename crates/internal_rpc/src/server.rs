use crate::{
    api::{IPFSDataType, InternalRpcApiServer, RpcResult},
    job_queue::{
        channel::{ServiceQueueChannel, ServiceReceiver, ServiceTransmitter},
        job::{ServiceJobApi, ServiceJobStatusResponse, ServiceJobType},
    },
};
use jsonrpsee::{
    core::async_trait,
    server::{ServerBuilder, ServerHandle},
};
use log::info;
use platform::services::*;
use service_config::ServiceConfig;
use std::{fmt::Debug, net::SocketAddr};
use web3_pkg::web3_store::Web3Store;

pub const MAX_RESPONSE_SIZE: u32 = 125_829_120;
pub const MAX_REQUEST_SIZE: u32 = 10240;

pub struct InternalRpcServer;
impl InternalRpcServer {
    /// Starts the RPC server which listens for internal calls.
    /// The server will continue to run until the handle is consumed.
    pub async fn start<
        T: ServiceTransmitter<J> + 'static,
        R: ServiceReceiver<J> + 'static,
        J: ServiceJobApi + Debug + 'static,
    >(
        service_config: &ServiceConfig,
        service_type: ServiceType,
    ) -> anyhow::Result<(ServerHandle, SocketAddr, R)> {
        let channel = ServiceQueueChannel::<T, R, J>::new();
        let rx = channel.rx;
        let tx = channel.tx;
        let rpc = InternalRpc::<T, J>::new(service_type, tx)?;
        let server = ServerBuilder::default()
            .max_response_body_size(MAX_RESPONSE_SIZE)
            .max_request_body_size(MAX_REQUEST_SIZE)
            .build(format!(
                "{}:{}",
                service_config.rpc_address, service_config.rpc_port
            ))
            .await?;

        info!(
            "Internal RPC service starting on {}:{}",
            &service_config.rpc_address, &service_config.rpc_port
        );

        let addr = server.local_addr()?;
        let handle = server.start(rpc.into_rpc())?;

        Ok((handle, addr, rx))
    }
}

/// Represents all information available to the server and client.
/// Calls to the [`InternalRpcApi`] rely on this structure.
struct InternalRpc<T: ServiceTransmitter<J>, J: ServiceJobApi + Debug> {
    /// An enum representing the service type. Compute, Storage, for example. More to come in the future.
    pub(crate) service_type: ServiceType,
    /// The time of the creation of the `InternalRpc`, used to get the uptime of a service.
    pub(crate) service_start: std::time::Instant,
    /// A bitmask of capabilities supported by a particular service.
    /// Subject to change, batteries not included.
    pub(crate) service_capabilities: ServiceCapabilities,
    /// The `CARGO_PKG_VERSION` as specified by `std::env`.
    pub(crate) version: VersionNumber,
    /// A transmitter that tracks service jobs with a built in queue.
    pub(crate) tx: T,
    marker: std::marker::PhantomData<J>,
}

impl<T: ServiceTransmitter<J>, J: ServiceJobApi + Debug> InternalRpc<T, J> {
    pub fn new(service_type: ServiceType, tx: T) -> anyhow::Result<Self> {
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
            tx,
            marker: std::marker::PhantomData,
        })
    }

    async fn retrieve_object(&self, cid: &str) -> RpcResult<Vec<u8>> {
        info!("Retrieving object '{}' from local IPFS instance.", &cid);
        let store = Web3Store::local()?;
        let obj = store.read_object(cid).await?;
        Ok(obj)
    }

    async fn retrieve_dag(&self, cid: &str) -> RpcResult<Vec<u8>> {
        info!("Retrieving DAG object '{}' from local IPFS instance.", &cid);
        let store = Web3Store::local()?;
        let obj = store.read_dag(cid).await?;
        Ok(obj)
    }

    async fn pin_object_ipfs(&self, cid: &str, recursive: bool) -> RpcResult<Vec<String>> {
        info!("Pinning object '{}' to local IPFS instance.", &cid);
        let store = Web3Store::local()?;
        let obj = store.pin_object(cid, recursive).await?;
        Ok(obj)
    }

    async fn is_pinned_obj(&self, cid: &str) -> RpcResult<bool> {
        info!(
            "Checking whether object '{}' is pinned to local IPFS instance.",
            &cid
        );
        let store = Web3Store::local()?;
        let is_pinned = store.is_pinned(cid).await?;
        Ok(is_pinned)
    }
}

#[async_trait]
impl<T: ServiceTransmitter<J> + 'static, J: ServiceJobApi + Debug + 'static> InternalRpcApiServer
    for InternalRpc<T, J>
{
    async fn status(&self) -> RpcResult<ServiceStatusResponse> {
        Ok(ServiceStatusResponse::from(self))
    }

    async fn queue_job(
        &self,
        cid: &str,
        kind: ServiceJobType,
        inputs: String,
    ) -> RpcResult<uuid::Uuid> {
        Ok(self.tx.send(cid, kind, inputs))
    }

    async fn job_status(&self, uuid: uuid::Uuid) -> RpcResult<Option<ServiceJobStatusResponse>> {
        Ok(self.tx.job_status(uuid))
    }

    async fn get_data(&self, cid: &str, data_type: IPFSDataType) -> RpcResult<Vec<u8>> {
        return match data_type {
            IPFSDataType::Object => self.retrieve_object(cid).await,
            IPFSDataType::Dag => self.retrieve_dag(cid).await,
        };
    }

    async fn pin_object(&self, cid: &str, recursive: bool) -> RpcResult<Vec<String>> {
        self.pin_object_ipfs(cid, recursive).await
    }
    async fn is_pinned(&self, cid: &str) -> RpcResult<bool> {
        self.is_pinned_obj(cid).await
    }
}

impl<'a, T: ServiceTransmitter<J>, J: ServiceJobApi + Debug> From<&'a InternalRpc<T, J>>
    for ServiceStatusResponse
{
    fn from(value: &'a InternalRpc<T, J>) -> Self {
        Self {
            service_type: value.service_type.clone(),
            service_capabilities: value.service_capabilities,
            service_implementation: "".to_string(),
            service_version: value.version.clone(),
            service_uptime: value.service_start.elapsed().as_secs(),
        }
    }
}
