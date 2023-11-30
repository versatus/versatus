use std::net::SocketAddr;

use jsonrpsee::{
    core::async_trait,
    proc_macros::rpc,
    server::{ServerBuilder, ServerHandle},
};
use platform::{services::*, sys::Utsname};
use service_config::ServiceConfig;

type RpcResult<T> = Result<T, jsonrpsee::core::Error>;

/// The methods available to the [`InternalRpcServer`] for both
/// the client and the server.
///
/// This is meant to be extensible to meet the needs of the `InternalRpcServer`.
#[rpc(server, client, namespace = "common")]
pub trait InternalRpcApi {
    /// Get the status of a job
    #[method(name = "status")]
    fn status(&self) -> RpcResult<()> {
        Ok(())
    }

    /// Get info about the current service
    #[method(name = "serviceStatusResponse")]
    fn service_status_response(&self) -> RpcResult<ServiceStatusResponse>;

    /// The name of this service definition
    #[method(name = "name")]
    fn name(&self) -> RpcResult<String>;

    /// The address to bind to for RPC calls
    #[method(name = "rpcAddress")]
    fn rpc_address(&self) -> RpcResult<String>;

    /// The port to bind to for RPC calls
    #[method(name = "rpcPort")]
    fn rpc_port(&self) -> RpcResult<u32>;

    /// A preshared key for authenticating RPC calls
    #[method(name = "preSharedKey")]
    fn pre_shared_key(&self) -> RpcResult<String>;

    /// A TLS private key for RPC transport privacy
    #[method(name = "tlsPrivateKeyFile")]
    fn tls_private_key_file(&self) -> RpcResult<String>;

    /// A TLS public certificate for RPC transport privacy
    #[method(name = "tlsPublicCertFile")]
    fn tls_public_cert_file(&self) -> RpcResult<String>;

    /// A TLS CA certificate for validating certificates
    #[method(name = "tlsCaCertFile")]
    fn tls_ca_cert_file(&self) -> RpcResult<String>;

    /// Prometheus exporter bind address
    #[method(name = "exporterAddress")]
    fn exporter_address(&self) -> RpcResult<String>;

    /// Prometheus exporter bind port
    #[method(name = "exporterPort")]
    fn exporter_port(&self) -> RpcResult<String>;
}

pub struct InternalRpcServer;
impl InternalRpcServer {
    /// Starts the RPC server which listens for internal calls.
    /// The server will continue to run until the handle is consumed.
    pub async fn start(
        service_config: &ServiceConfig,
        service_type: ServiceType,
    ) -> anyhow::Result<(ServerHandle, SocketAddr)> {
        let rpc = InternalRpc::new(service_config, service_type)?;
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
    pub(crate) service_config: ServiceConfig,
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
    pub fn new(service_config: &ServiceConfig, service_type: ServiceType) -> anyhow::Result<Self> {
        let extra_service_capabilities = ServiceCapabilities::try_from(Utsname::new()?)?;
        Ok(Self {
            service_config: service_config.clone(),
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
}

#[async_trait]
impl InternalRpcApiServer for InternalRpc {
    fn service_status_response(&self) -> RpcResult<ServiceStatusResponse> {
        Ok(ServiceStatusResponse::from(self))
    }

    fn name(&self) -> RpcResult<String> {
        Ok(self.service_config.name.clone())
    }

    fn rpc_address(&self) -> RpcResult<String> {
        Ok(self.service_config.rpc_address.clone())
    }

    fn rpc_port(&self) -> RpcResult<u32> {
        Ok(self.service_config.rpc_port)
    }

    fn pre_shared_key(&self) -> RpcResult<String> {
        Ok(self.service_config.pre_shared_key.clone())
    }

    fn tls_private_key_file(&self) -> RpcResult<String> {
        Ok(self.service_config.tls_private_key_file.clone())
    }

    fn tls_public_cert_file(&self) -> RpcResult<String> {
        Ok(self.service_config.tls_public_cert_file.clone())
    }

    fn tls_ca_cert_file(&self) -> RpcResult<String> {
        Ok(self.service_config.tls_ca_cert_file.clone())
    }

    fn exporter_address(&self) -> RpcResult<String> {
        Ok(self.service_config.exporter_address.clone())
    }

    fn exporter_port(&self) -> RpcResult<String> {
        Ok(self.service_config.exporter_port.clone())
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
