use jsonrpsee::core::Error;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::SubscriptionResult;
use primitives::{NodeType, PublicKey};
use serde::{Deserialize, Serialize};
use vrrb_core::txn::{TxAmount, TxNonce, TxPayload, TxSignature, TxToken};

pub type ExampleHash = [u8; 32];
pub type ExampleStorageKey = Vec<u8>;
pub type FullStateSnapshot = Vec<(Vec<u8>, Vec<u8>)>;
pub type FullMempoolSnapshot = Vec<(Vec<u8>, Vec<u8>)>;

#[derive(Serialize, Deserialize)]
pub struct CreateTxnArgs {
    pub sender_address: String,
    pub sender_public_key: PublicKey,
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
}
