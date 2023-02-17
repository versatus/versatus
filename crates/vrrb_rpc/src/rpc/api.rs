use std::{collections::HashMap, net::SocketAddr};

use async_trait::async_trait;
use jsonrpsee::{core::Error, proc_macros::rpc, types::SubscriptionResult};
use primitives::{
    Address,
    NodeType,
    PublicKey,
    SerializedPublicKey,
    SerializedPublicKeyString,
    TransactionDigest,
};
use serde::{Deserialize, Serialize};
use storage::vrrbdb::VrrbDbReadHandle;
use vrrb_core::{
    account::Account,
    txn::{TxAmount, TxNonce, TxPayload, TxSignature, TxToken, Txn},
};

pub type ExampleHash = [u8; 32];
pub type ExampleStorageKey = Vec<u8>;
pub type FullStateSnapshot = HashMap<Address, Account>;
pub type FullMempoolSnapshot = Vec<Txn>;

#[derive(Serialize, Deserialize)]
pub struct CreateTxnArgs {
    pub sender_address: String,
    pub sender_public_key: SerializedPublicKey,
    pub receiver_address: String,
    pub token: Option<String>,
    pub amount: TxAmount,
    pub payload: Option<TxPayload>,
    pub signature: TxSignature,
    pub nonce: TxNonce,
}

#[rpc(server, client, namespace = "state")]
pub trait Rpc {
    /// Returns a full list of all accounts within state
    #[method(name = "getFullState")]
    async fn get_full_state(&self) -> Result<FullStateSnapshot, Error>;

    /// Returns a full list of transactions pending to be confirmed
    #[method(name = "getFullMempool")]
    async fn get_full_mempool(&self) -> Result<FullMempoolSnapshot, Error>;

    /// Returns the node type this client is connected to
    #[method(name = "getNodeType")]
    async fn get_node_type(&self) -> Result<NodeType, Error>;

    #[method(name = "createTxn")]
    async fn create_txn(&self, args: CreateTxnArgs) -> Result<(), Error>;

    #[method(name = "getTransaction")]
    async fn get_transaction(&self, transaction_digest: TransactionDigest) -> Result<(), Error>;

    #[method(name = "listTransactions")]
    async fn list_transactions(&self) -> Result<(), Error>;

    #[method(name = "createAccount")]
    async fn create_account(&self, account: Account) -> Result<(), Error>;

    #[method(name = "updateAccount")]
    async fn update_account(&self, account: Account) -> Result<(), Error>;
}
