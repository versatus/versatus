use axum::{Extension, http::StatusCode, Json, Router};

use serde::Deserialize;
use serde_json::json;
use std::{convert::Infallible, net::SocketAddr};
use std::sync::Arc;
use axum::routing::post;
use tokio::spawn;
use tokio::sync::Mutex;
use primitives::Address;
use vrrb_core::transactions::{RpcTransactionDigest, Token};
use wallet::v2::{Wallet, WalletConfig, WalletError};

#[derive(Deserialize)]
struct FaucetRequest {
    account: String,
}

pub struct FaucetConfig {
    pub rpc_server_address: SocketAddr,
    pub server_port: u16,
    pub secret_key: String,
    pub transfer_amount: u64,
}

pub struct Faucet {
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
        .await
        .map_err(|err| {
            eprintln!("Unable to send transaction: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    ;
    //
    Ok(Json::from(digest))
}

impl Faucet {
    pub async fn new(config: FaucetConfig) -> Result<Self, WalletError> {

        let wallet = Wallet::restore_from_private_key(
            config.secret_key.clone(),
            config.rpc_server_address,
        ).await?;

        eprintln!("wallet: {:?}", wallet);

        let faucet = Faucet {
            config,
            wallet: Arc::new(Mutex::new(wallet)),
        };

        Ok(faucet)
    }
    pub async fn start(self) -> Result<(), axum::Error> {

        let app = Router::new()
            .route("/drip", post(drip))
            .layer(Extension(self.wallet));

        let addr = SocketAddr::from(([127, 0, 0, 1], self.config.server_port));
        println!("Server started at http://{}", addr);
        axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();

        Ok(())
    }
}