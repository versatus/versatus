use crate::job_queue::job::{ServiceJobStatusResponse, ServiceJobType};
use jsonrpsee::proc_macros::rpc;
use platform::services::ServiceStatusResponse;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::str::FromStr;

pub(crate) type RpcResult<T> = Result<T, jsonrpsee::core::Error>;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum IPFSDataType {
    Object,
    Dag,
}

impl FromStr for IPFSDataType {
    type Err = String; // Define the error type

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lowercase_input = s.to_lowercase();
        match lowercase_input.as_str() {
            "object" => Ok(IPFSDataType::Object),
            "dag" => Ok(IPFSDataType::Dag),
            _ => Err("Invalid IPFSDataType".to_string()),
        }
    }
}

/// The methods available to the [`InternalRpcServer`] for both
/// the client and the server.
///
/// This is meant to be extensible to meet the needs of the `InternalRpcServer`.
#[rpc(server, client, namespace = "common")]
#[rpc(response_max_size = 104857600)] // Set maximum response size to 100MB
pub trait InternalRpcApi {
    /// Get info about the current service
    #[method(name = "status")]
    async fn status(&self) -> RpcResult<ServiceStatusResponse>;

    /// Add a job to the job queue via the queue channel's transmitter
    #[method(name = "queue_job")]
    async fn queue_job(
        &self,
        cid: &str,
        kind: ServiceJobType,
        inputs: String,
    ) -> RpcResult<uuid::Uuid>;

    /// Get the status of a job
    #[method(name = "job_status")]
    async fn job_status(&self, uuid: uuid::Uuid) -> RpcResult<Option<ServiceJobStatusResponse>>;

    #[method(name = "get_object")]
    async fn get_data(&self, cid: &str, data_type: IPFSDataType) -> RpcResult<Vec<u8>>;

    #[method(name = "pin_object")]
    async fn pin_object(&self, cid: &str, recursive: bool) -> RpcResult<Vec<String>>;

    #[method(name = "is_pinned")]
    async fn is_pinned(&self, cid: &str) -> RpcResult<bool>;
}
