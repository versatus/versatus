use crate::api::RpcResult;
use jsonrpsee::ws_client::WsClientBuilder;

// Stub for the client request cli
#[derive(Debug)]
pub struct ClientRequest;

async fn _run(request_opts: ClientRequest, server_url_with_port: &str) -> RpcResult<()> {
    // Connect to the JSON-RPC server using WebSocket
    let client = WsClientBuilder::default()
        .build(&server_url_with_port)
        .await?;

    // Example: Call a JSON-RPC method
    // This will be where we make requests from the server
    // after establishing a connection
    if client.is_connected() {
        println!("connection to server established");
        dbg!(&request_opts);
        Ok(())
    } else {
        Err(jsonrpsee::core::Error::Custom(format!(
            "failed to establish connection to server at {}",
            server_url_with_port
        )))
    }
}

#[tokio::test]
async fn test_client() {
    assert!(_run(ClientRequest, "wss://localhost:443").await.is_ok());
}
