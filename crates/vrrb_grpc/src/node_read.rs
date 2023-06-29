include!("gen/mod.rs");

use std::collections::HashMap;

use events::EventPublisher;
use mempool::MempoolReadHandleFactory;
use node_read_service::v1::{
    node_read_service_server::{NodeReadService, NodeReadServiceServer},
    GetFullMempoolRequest,
    GetFullMempoolResponse,
    GetNodeTypeRequest,
    GetNodeTypeResponse,
    Token as NodeToken,
    TransactionRecord,
};
use primitives::NodeType;
use serde::{Deserialize, Serialize};
use storage::vrrbdb::VrrbDbReadHandle;
use tonic::{Request, Response, Status};
use vrrb_core::txn::{Token, TxAmount, TxNonce, TxTimestamp, Txn};

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

#[derive(Debug)]
pub struct NodeRead {
    pub node_type: NodeType,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
    pub events_tx: EventPublisher,
}

impl NodeRead {
    pub fn init(self) -> NodeReadServiceServer<NodeRead> {
        NodeReadServiceServer::new(self)
    }
}

#[tonic::async_trait]
impl NodeReadService for NodeRead {
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

        Ok(Response::new(response))
    }
}