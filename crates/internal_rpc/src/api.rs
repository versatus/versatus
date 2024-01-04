use jsonrpsee::proc_macros::rpc;
use platform::services::ServiceStatusResponse;
use serde::{Deserialize, Serialize};
pub(crate) type RpcResult<T> = Result<T, jsonrpsee::core::Error>;

#[derive(Serialize, Deserialize)]
pub enum IPFSDataType {
    Object,
    Dag,
}
/// The methods available to the [`InternalRpcServer`] for both
/// the client and the server.
///
/// This is meant to be extensible to meet the needs of the `InternalRpcServer`.
#[rpc(server, client, namespace = "common")]
pub trait InternalRpcApi {
    /// Get info about the current service
    #[method(name = "status")]
    async fn status(&self) -> RpcResult<ServiceStatusResponse>;

    #[method(name = "get_object")]
    async fn get_data(&self, cid: &str, data_type: IPFSDataType) -> RpcResult<Vec<(String, Vec<u8>)>>;

    #[method(name = "pin_object")]
    async fn pin_object(&self, cid: &str, recursive: bool) -> RpcResult<Vec<String>>;

    #[method(name = "is_pinned")]
    async fn is_pinned(&self, cid: &str) -> RpcResult<bool>;
}
