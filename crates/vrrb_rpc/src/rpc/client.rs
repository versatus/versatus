use std::net::SocketAddr;

use jsonrpsee::{core::client::Client, ws_client::WsClientBuilder};

use crate::ApiError;

pub async fn create_client(server_url: SocketAddr) -> crate::Result<Client> {
    let jsonrpc_url = format!("ws://{server_url}");

    let client = WsClientBuilder::default()
        .build(&jsonrpc_url)
        .await
        .map_err(|err| ApiError::Other(format!("unable to start JSON-RPC server: {err}")))?;

    Ok(client)
}
