use std::collections::HashMap;

use block::block::Block;
use block::ClaimHash;
use jsonrpsee::{core::Error as RpseeError, proc_macros::rpc};
use primitives::{Address, NodeType, Round};
use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use storage::vrrbdb::Claims;
use vrrb_config::bootstrap_quorum::QuorumMembershipConfig;
use vrrb_core::account::Account;
use vrrb_core::node_health_report::NodeHealthReport;
use vrrb_core::transactions::{
    RpcTransactionDigest, Token, Transaction, TransactionKind, TxAmount, TxNonce, TxTimestamp,
};

use crate::rpc::SignOpts;

pub type ExampleHash = [u8; 32];
pub type ExampleStorageKey = Vec<u8>;
pub type FullStateSnapshot = HashMap<Address, Account>;
pub type FullMempoolSnapshot = Vec<RpcTransactionRecord>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullMempoolSnapshotResponse {
    data: Vec<TransactionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcTransactionRecord {
    pub id: RpcTransactionDigest,
    pub timestamp: TxTimestamp,
    pub sender_address: Address,
    pub sender_public_key: PublicKey,
    pub receiver_address: Address,
    pub token: Token,
    pub amount: TxAmount,
    pub signature: String,
    pub validators: HashMap<String, bool>,
    pub nonce: TxNonce,
}

impl From<TransactionKind> for RpcTransactionRecord {
    fn from(txn: TransactionKind) -> Self {
        Self {
            id: txn.id().digest_string(),
            timestamp: txn.timestamp(),
            sender_address: txn.sender_address(),
            sender_public_key: txn.sender_public_key(),
            receiver_address: txn.receiver_address(),
            token: txn.token(),
            amount: txn.amount(),
            signature: txn.signature().to_string(),
            validators: txn.validators().unwrap_or_default(),
            nonce: txn.nonce(),
        }
    }
}

#[rpc(server, client, namespace = "state")]
#[async_trait]
pub trait RpcApi {
    /// Returns a full list of all accounts within state
    #[method(name = "getFullState")]
    async fn get_full_state(&self) -> Result<FullStateSnapshot, RpseeError>;

    /// Returns a full list of transactions pending to be confirmed
    #[method(name = "getFullMempool")]
    async fn get_full_mempool(&self) -> Result<FullMempoolSnapshot, RpseeError>;

    /// Returns the node type this client is connected to
    #[method(name = "getNodeType")]
    async fn get_node_type(&self) -> Result<NodeType, RpseeError>;

    /// Create a new transaction
    #[method(name = "createTxn")]
    async fn create_txn(&self, txn: TransactionKind) -> Result<RpcTransactionRecord, RpseeError>;

    /// Get a transaction from state
    #[method(name = "getTransaction")]
    async fn get_transaction(
        &self,
        transaction_digest: RpcTransactionDigest,
    ) -> Result<RpcTransactionRecord, RpseeError>;

    /// List a group of transactions
    #[method(name = "listTransactions")]
    async fn list_transactions(
        &self,
        digests: Vec<RpcTransactionDigest>,
    ) -> Result<HashMap<RpcTransactionDigest, RpcTransactionRecord>, RpseeError>;

    #[method(name = "createAccount")]
    async fn create_account(&self, address: Address, account: Account) -> Result<(), RpseeError>;

    #[method(name = "updateAccount")]
    async fn update_account(&self, account: Account) -> Result<(), RpseeError>;

    #[method(name = "getAccount")]
    async fn get_account(&self, address: Address) -> Result<Account, RpseeError>;

    #[method(name = "faucetDrip")]
    async fn faucet_drip(&self, address: Address) -> Result<(), RpseeError>;

    #[method(name = "signTransaction")]
    async fn sign_transaction(&self, sign_opts: SignOpts) -> Result<String, RpseeError>;

    #[method(name = "getRound")]
    async fn get_round(&self) -> Result<Round, RpseeError>;

    #[method(name = "getBlocks")]
    async fn get_blocks(&self) -> Result<Vec<Block>, RpseeError>;

    #[method(name = "getProgram")]
    async fn get_program(&self) -> Result<(), RpseeError>;

    #[method(name = "callProgram")]
    async fn call_program(&self) -> Result<(), RpseeError>;

    #[method(name = "getTransactionCount")]
    async fn get_transaction_count(&self, account: Address) -> Result<usize, RpseeError>;

    #[method(name = "getNodeHealth")]
    async fn get_node_health(&self) -> Result<NodeHealthReport, RpseeError>;

    #[method(name = "getClaimsByAccountId")]
    async fn get_claims_by_account_id(&self, address: Address) -> Result<Claims, RpseeError>;

    #[method(name = "getClaimHashes")]
    async fn get_claim_hashes(&self) -> Result<Vec<ClaimHash>, RpseeError>;

    #[method(name = "getClaims")]
    async fn get_claims(&self, claim_hashes: Vec<ClaimHash>) -> Result<Claims, RpseeError>;

    #[method(name = "getMembershipConfig")]
    async fn get_membership_config(&self) -> Result<QuorumMembershipConfig, RpseeError>;

    #[method(name = "getLastBlock")]
    async fn get_last_block(&self) -> Result<Block, RpseeError>;
}
