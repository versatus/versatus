use jsonrpsee::proc_macros::rpc;
use platform::services::ServiceStatusResponse;
use service_config::ServiceConfig;

pub(crate) type RpcResult<T> = Result<T, jsonrpsee::core::Error>;

/// The methods available to the [`InternalRpcServer`] for both
/// the client and the server.
///
/// This is meant to be extensible to meet the needs of the `InternalRpcServer`.
#[rpc(server, client, namespace = "common")]
pub trait InternalRpcApi {
    /// Get the status of a job
    #[method(name = "status")]
    async fn status(&self) -> RpcResult<()> {
        Ok(())
    }

    /// Get info about the current service
    #[method(name = "serviceStatusResponse")]
    async fn service_status_response(&self) -> RpcResult<ServiceStatusResponse>;

    /// Returns a copy of the [`ServiceConfig`] the server was constructed with
    #[method(name = "serviceConfig")]
    async fn service_config(&self) -> RpcResult<ServiceConfig>;
}
