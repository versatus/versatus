use std::{collections::HashMap, net::SocketAddr};

use async_trait::async_trait;
use jsonrpsee::{core::Error, proc_macros::rpc, types::SubscriptionResult};
use primitives::{Address, NodeType, SerializedPublicKey};
use serde::{Deserialize, Serialize};
use vrrb_core::{
    account::Account,
    txn::{NewTxnArgs, Txn},
};

pub type ExampleHash = [u8; 32];
pub type ExampleStorageKey = Vec<u8>;
pub type FullStateSnapshot = HashMap<Address, Account>;
pub type FullMempoolSnapshot = Vec<RpcTransactionRecord>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    //
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullMempoolSnapshotResponse {
    //
}

pub type RpcTransactionDigest = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcTransactionRecord {
    //
}

impl From<Txn> for RpcTransactionRecord {
    fn from(txn: Txn) -> Self {
        Self {}
    }
}

#[rpc(server, client, namespace = "state")]
#[async_trait]
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

    /// Create a new transaction
    #[method(name = "createTxn")]
    async fn create_txn(&self, args: NewTxnArgs) -> Result<Txn, Error>;

    /// Get a transaction from state
    #[method(name = "getTransaction")]
    async fn get_transaction(
        &self,
        transaction_digest: RpcTransactionDigest,
    ) -> Result<RpcTransactionRecord, Error>;

    /// List a group of transactions
    #[method(name = "listTransactions")]
    async fn list_transactions(
        &self,
        digests: Vec<RpcTransactionDigest>,
    ) -> Result<HashMap<RpcTransactionDigest, RpcTransactionRecord>, Error>;

    #[method(name = "createAccount")]
    async fn create_account(&self, address: Address, account: Account) -> Result<(), Error>;

    #[method(name = "updateAccount")]
    async fn update_account(&self, account: Account) -> Result<(), Error>;

    #[method(name = "getAccount")]
    async fn get_account(&self, address: Address) -> Result<Account, Error>;

    //#[method(name = "faucetDrip")]
    //async fn faucet_drip(&self, address: Address) -> Result<(), Error>;
}
