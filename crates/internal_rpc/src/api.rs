use jsonrpsee::proc_macros::rpc;
use platform::services::ServiceStatusResponse;

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
