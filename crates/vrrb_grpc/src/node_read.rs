include!("gen/mod.rs");

use std::{collections::HashMap, str::FromStr};

use events::EventPublisher;
use mempool::MempoolReadHandleFactory;
use node_read_service::v1::{
    node_read_service_server::{NodeReadService, NodeReadServiceServer},
    Account as RpcAccount,
    FullStateSnapshotRequest,
    FullStateSnapshotResponse,
    GetAccountRequest,
    GetAccountResponse,
    GetFullMempoolRequest,
    GetFullMempoolResponse,
    GetNodeTypeRequest,
    GetNodeTypeResponse,
    GetTransactionRequest,
    GetTransactionResponse,
    ListTransactionsRequest,
    ListTransactionsResponse,
    Token as NodeToken,
    TransactionRecord,
};
use primitives::{Address, NodeType};
use serde::{Deserialize, Serialize};
use storage::vrrbdb::VrrbDbReadHandle;
use tonic::{Request, Response, Status};
use vrrb_core::{
    account::Account,
    txn::{Token, TransactionDigest, TxAmount, TxNonce, TxTimestamp, Txn},
};

impl From<HashMap<Address, Account>> for FullStateSnapshotResponse {
    fn from(map: HashMap<Address, Account>) -> Self {
        let mut snapshot: HashMap<String, RpcAccount> = HashMap::new();

        map.iter().for_each(|(address, account)| {
            let rpc_account: RpcAccount = RpcAccount::from(account);
            snapshot.insert(address.to_string(), rpc_account);
        });

        Self {
            full_state_snapshot: snapshot,
        }
    }
}

pub type RpcTransactionDigest = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcTransactionRecord {
    pub id: RpcTransactionDigest,
    pub timestamp: TxTimestamp,
    pub sender_address: String,
    pub sender_public_key: String,
    pub receiver_address: String,
    pub token: Token,
    pub amount: TxAmount,
    pub signature: String,
    pub validators: HashMap<String, bool>,
    pub nonce: TxNonce,
}

impl From<Txn> for RpcTransactionRecord {
    fn from(txn: Txn) -> Self {
        Self {
            id: txn.digest().to_string(),
            timestamp: txn.timestamp(),
            sender_address: txn.sender_address().to_string(),
            sender_public_key: txn.sender_public_key().to_string(),
            receiver_address: txn.receiver_address().to_string(),
            token: txn.token(),
            amount: txn.amount(),
            signature: txn.signature().to_string(),
            validators: txn.validators(),
            nonce: txn.nonce(),
        }
    }
}

impl From<RpcTransactionRecord> for TransactionRecord {
    fn from(rpc_transaction_record: RpcTransactionRecord) -> Self {
        let amount = rpc_transaction_record.amount as u64;
        let nonce = rpc_transaction_record.nonce as u64;
        let token = NodeToken {
            name: rpc_transaction_record.token.name,
            symbol: rpc_transaction_record.token.symbol,
            decimals: rpc_transaction_record.token.decimals as u32,
        };

        Self {
            id: rpc_transaction_record.id,
            timestamp: rpc_transaction_record.timestamp,
            sender_address: rpc_transaction_record.sender_address,
            sender_public_key: rpc_transaction_record.sender_public_key,
            receiver_address: rpc_transaction_record.receiver_address,
            token: Some(token),
            amount,
            signature: rpc_transaction_record.signature,
            validators: rpc_transaction_record.validators,
            nonce,
        }
    }
}

impl From<&Account> for RpcAccount {
    fn from(account: &Account) -> Self {
        Self {
            hash: account.hash.clone(),
            account_nonce: account.nonce as u64,
            credits: account.credits as u64,
            debits: account.debits as u64,
            storage: account.storage.clone().unwrap(),
            code: account.code.clone().unwrap(),
            pubkey: String::from_utf8(account.pubkey.clone()).unwrap(),
            digests: None, // TOOD: implement
            created_at: account.created_at,
            updated_at: account.updated_at.unwrap(),
        }
    }
}

impl From<&Account> for GetAccountResponse {
    fn from(account: &Account) -> Self {
        let rpc_account = RpcAccount::from(account);

        Self {
            account: Some(rpc_account),
        }
    }
}

#[derive(Debug)]
pub struct NodeRead {
    pub node_type: NodeType,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
    pub events_tx: EventPublisher,
}

impl NodeRead {
    pub fn init(self) -> NodeReadServiceServer<NodeRead> {
        let node_service = NodeReadServiceServer::new(self);
        return node_service;
    }
}

#[tonic::async_trait]
impl NodeReadService for NodeRead {
    async fn get_full_state(
        &self,
        _request: Request<FullStateSnapshotRequest>,
    ) -> Result<Response<FullStateSnapshotResponse>, Status> {
        let full_mempool_state = self.vrrbdb_read_handle.state_store_values();
        let snapshot = FullStateSnapshotResponse::from(full_mempool_state);

        Ok(Response::new(snapshot))
    }

    async fn get_full_mempool(
        &self,
        _request: Request<GetFullMempoolRequest>,
    ) -> Result<Response<GetFullMempoolResponse>, Status> {
        let transaction_records = self
            .mempool_read_handle_factory
            .values()
            .iter()
            .map(|txn| TransactionRecord::from(RpcTransactionRecord::from(txn.clone())))
            .collect();

        let response = GetFullMempoolResponse {
            transaction_records,
        };

        return Ok(Response::new(response));
    }

    async fn get_node_type(
        &self,
        _request: Request<GetNodeTypeRequest>,
    ) -> Result<Response<GetNodeTypeResponse>, Status> {
        let response = GetNodeTypeResponse {
            id: (self.node_type as i32).to_string(),
            result: self.node_type.to_string(),
        };
        Ok(Response::new(response))
    }

    async fn get_transaction(
        &self,
        request: Request<GetTransactionRequest>,
    ) -> Result<Response<GetTransactionResponse>, Status> {
        let transaction_request = request.into_inner();

        let parsed_digest = RpcTransactionDigest::from(&transaction_request.rpc_transaction_digest)
            .parse::<TransactionDigest>()
            .map_err(|e| Status::invalid_argument(format!("Invalid Argument: {}", e)))?;

        let values = self.vrrbdb_read_handle.transaction_store_values();
        let value = values.get(&parsed_digest);

        match value {
            Some(txn) => {
                let txn_record = RpcTransactionRecord::from(txn.clone());
                let response = GetTransactionResponse {
                    transaction_record: Some(TransactionRecord::from(txn_record)),
                };

                Ok(Response::new(response))
            },
            None => return Err(Status::not_found("Transaction does not exist")),
        }
    }

    async fn list_transactions(
        &self,
        request: Request<ListTransactionsRequest>,
    ) -> Result<Response<ListTransactionsResponse>, Status> {
        let digests = request.into_inner().digests;

        let mut transactions = ListTransactionsResponse::default();

        digests
            .iter()
            .try_for_each(|digest_string| -> Result<(), Status> {
                let parsed_digest = digest_string
                    .parse::<TransactionDigest>()
                    .map_err(|e| Status::invalid_argument(format!("Invalid Argument: {}", e)))?;

                if let Some(txn) = self
                    .vrrbdb_read_handle
                    .transaction_store_values()
                    .get(&parsed_digest)
                {
                    let rpc_txn_record = RpcTransactionRecord::from(txn.clone());
                    let txn_record = TransactionRecord::from(rpc_txn_record);

                    transactions
                        .transactions
                        .insert(txn.digest().to_string(), txn_record);
                }

                Ok(())
            })?;

        Ok(Response::new(transactions))
    }

    async fn get_account(
        &self,
        request: Request<GetAccountRequest>,
    ) -> Result<Response<GetAccountResponse>, Status> {
        let public_key = Address::from_str(&request.into_inner().address)
            .map_err(|e| Status::invalid_argument(format!("Invalid Argument: {}", e)))?;

        let values = self.vrrbdb_read_handle.state_store_values();
        let value = values.get(&public_key);

        match value {
            Some(account) => {
                let response = GetAccountResponse::from(account);
                return Ok(Response::new(response));
            },
            None => return Err(Status::not_found("Account does not exist")),
        }
    }
}
