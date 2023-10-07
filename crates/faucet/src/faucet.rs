use axum::{Extension, http::StatusCode, Json, Router};

use serde::Deserialize;
use serde_json::json;
use std::{convert::Infallible, net::SocketAddr};
use std::sync::Arc;
use axum::routing::post;
use tokio::spawn;
use tokio::sync::Mutex;
use primitives::{Address, SecretKey};
use vrrb_core::transactions::{RpcTransactionDigest, Token};
use wallet::v2::{Wallet, WalletConfig, WalletError};

#[derive(Deserialize)]
struct FaucetRequest {
    account: String,
}

pub struct FaucetConfig {
    pub rpc_server_address: SocketAddr,
    pub server_port: u16,
    pub secret_key: SecretKey,
    pub transfer_amount: u64,
    // Add other configuration parameters if needed.
}

struct Faucet {
    config: FaucetConfig,
    wallet: Arc<Mutex<Wallet>>,
}
async fn drip(
    Extension(wallet): Extension<Arc<Mutex<Wallet>>>,
    Json(req): Json<FaucetRequest>,
) -> Result<Json<RpcTransactionDigest>, StatusCode> {
    let recipient: Address = req.account.parse().unwrap();

    let timestamp = chrono::Utc::now().timestamp();

    // Locking wallet for mutation.
    let mut wallet = wallet.lock().await;

    let digest = wallet
        .send_transaction(
            0,
            recipient,
            10,
            Token::default(),
            timestamp
        )
        // .map_err(|err| {
        //     eprintln!("Unable to send transaction: {}", err);
        //     StatusCode::INTERNAL_SERVER_ERROR
        // })?;
    ;
    //
    // Ok(Json::from(digest))
    Ok(Json::from("digest".to_string()))
}

impl Faucet {
    pub async fn new(config: FaucetConfig) -> Result<Self, WalletError> {
        let wallet_config = WalletConfig {
            rpc_server_address: config.rpc_server_address,
            secret_key: config.secret_key,
            ..Default::default()
        };

        let wallet = Wallet::new(wallet_config).await?;

        let faucet = Faucet {
            config,
            wallet: Arc::new(Mutex::new(wallet)),
        };

        Ok(faucet)
    }
    pub async fn start(self) -> Result<(), axum::Error> {

        let app = Router::new()
            .route("/faucet", post(drip))
            .layer(Extension(self.wallet));

        let addr = SocketAddr::from(([127, 0, 0, 1], self.config.server_port));
        println!("Server started at http://{}", addr);
        axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();

        Ok(())
    }
}
