use tokio::sync::mpsc::channel;
use vrrb_rpc::http::*;

#[tokio::test]
async fn server_starts_and_stops() {
    let config = HttpApiServerConfig {
        address: "127.0.0.1:0".into(),
        api_title: "Node HTTP API".into(),
        api_version: "1.0".into(),
        server_timeout: None,
    };

    let api = HttpApiServer::new(config).unwrap();

    let (ctrl_tx, mut ctrl_rx) = channel(1);

    let server_handle = tokio::spawn(async move {
        api.start(&mut ctrl_rx).await.unwrap();
    });

    ctrl_tx.send(()).await.unwrap();
    server_handle.await.unwrap();
}
