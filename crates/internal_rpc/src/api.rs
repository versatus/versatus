use jsonrpsee::proc_macros::rpc;
use platform::services::ServiceStatusResponse;

pub(crate) type RpcResult<T> = Result<T, jsonrpsee::core::Error>;

/// The methods available to the [`InternalRpcServer`] for both
/// the client and the server.
///
/// This is meant to be extensible to meet the needs of the `InternalRpcServer`.
#[rpc(server, client, namespace = "common")]
pub trait InternalRpcApi {
    /// Get info about the current service
    #[method(name = "status")]
    async fn status(&self) -> RpcResult<ServiceStatusResponse>;
}
