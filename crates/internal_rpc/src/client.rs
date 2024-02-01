use crate::api::RpcResult;
use crate::server::MAX_RESPONSE_SIZE;
use jsonrpsee::core::client::Client;
use jsonrpsee::ws_client::WsClientBuilder;
use std::net::SocketAddr;

/// The websocket internal RPC client used for
/// requesting services and obtaining responses
/// from the `InternalRpcServer`.
///
/// To interact with the server, use the methods
/// available on the `InternalRpcApi` interface.
pub struct InternalRpcClient(pub Client);
impl InternalRpcClient {
    /// Accepts a URL to a server, and attempts to build a client bound to that URL.
    /// The URL to the server MUST include the port.
    pub async fn new(socket: SocketAddr) -> RpcResult<Self> {
        let client = WsClientBuilder::default()
            .max_request_body_size(MAX_RESPONSE_SIZE)
            .build(format!("ws://{socket}"))
            .await?;

        if client.is_connected() {
            println!("connection to server established");
            Ok(InternalRpcClient(client))
        } else {
            Err(jsonrpsee::core::Error::Custom(format!(
                "failed to establish connection to server at {}",
                socket
            )))
        }
    }
    pub fn is_connected(&self) -> bool {
        self.0.is_connected()
    }
}
