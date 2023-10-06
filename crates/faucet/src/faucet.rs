use axum::{handler::post, http::StatusCode, Router};
use serde::Deserialize;
use serde_json::json;
use std::{convert::Infallible, net::SocketAddr};
use tokio::spawn;
use wallet::v2::{Wallet, WalletConfig};

#[derive(Deserialize)]
struct FaucetRequest {
    account: String,
}

pub struct FaucetConfig {
    pub rpc_server_address: String,
    pub server_port: u16,
    pub wallet_mnemonic: String,
    // Add other configuration parameters if needed.
}

struct Faucet {
    config: FaucetConfig,
    wallet: Wallet,
}

async fn faucet(req: axum::extract::Json<FaucetRequest>) -> Result<axum::response::Json<serde_json::Value>, StatusCode> {
    println!("Setting RPC server address: {}", req.rpc_server);

    // Here, you should call your crate's function to handle transactions.
    // spawn(your_crate::start_transaction_handler(req.rpc_server));

    // Simulating the process with a dummy async block.
    spawn(async {
        println!("Dummy transaction handler started with RPC: {}", req.rpc_server);
    });

    Ok(axum::response::Json(json!({"message": "RPC Server Set"})))
}

impl Faucet {
    pub async fn start(config: FaucetConfig) -> Result<(), Box<dyn std::error::Error>> {
        let mut wallet_config = WalletConfig::default();
        wallet_config.rpc_server_address = config.rpc_server_address;

        let mut wallet = Wallet::new(wallet_config).await.unwrap();

        let app = Router::new()
            .route("/faucet", post(faucet))
            .handle_error(|error: axum::Error| {
                let (status, _response_body) = (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {:?}", error),
                );
                Ok::<_, Infallible>((status, _response_body))
            });

        let addr = SocketAddr::from(([127, 0, 0, 1], config.server_port));
        println!("Server started at http://{}", addr);
        axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
    }
}
