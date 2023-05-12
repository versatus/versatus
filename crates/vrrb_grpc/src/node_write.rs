include!("gen/mod.rs");

use std::{collections::HashMap, str::FromStr};

use events::{Event, EventPublisher};
use mempool::MempoolReadHandleFactory;
use node_write_service::v1::{
    node_write_service_server::{NodeWriteService, NodeWriteServiceServer},
    CreateTransactionRequest,
    Token as NodeToken,
    TransactionRecord,
};
use primitives::NodeType;
use secp256k1::{ecdsa::Signature, PublicKey};
use serde::{Deserialize, Serialize};
use storage::vrrbdb::VrrbDbReadHandle;
use tonic::{Request, Response, Status};
use vrrb_core::txn::{NewTxnArgs, Token, TxAmount, TxNonce, TxTimestamp, Txn};

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
            sender_address: txn.sender_address(),
            sender_public_key: txn.sender_public_key().to_string(),
            receiver_address: txn.receiver_address(),
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

// TODO: From TXRequest and implement the same logic, do not implement the
// native from trait, create "your own"

impl From<CreateTransactionRequest> for NewTxnArgs {
    fn from(create_transaction_request: CreateTransactionRequest) -> Self {
        let pub_key = PublicKey::from_str(&create_transaction_request.sender_public_key).unwrap();
        let signature = Signature::from_str(&create_transaction_request.signature).unwrap();
        let amount = create_transaction_request.amount as u128;
        let nonce = create_transaction_request.nonce as u128;
        let request_token = create_transaction_request
            .token
            .expect("Token to be provided");
        let token: Token = Token {
            name: request_token.name,
            symbol: request_token.symbol,
            decimals: request_token.decimals as u8,
        };

        Self {
            timestamp: create_transaction_request.timestamp,
            sender_address: create_transaction_request.sender_address,
            sender_public_key: pub_key,
            receiver_address: create_transaction_request.receiver_address,
            token: Some(token),
            amount,
            signature,
            validators: Some(create_transaction_request.validators),
            nonce,
        }
    }
}

#[derive(Debug)]
pub struct NodeWrite {
    pub node_type: NodeType,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
    pub events_tx: EventPublisher,
}

impl NodeWrite {
    pub fn init(self) -> NodeWriteServiceServer<NodeWrite> {
        let node_service = NodeWriteServiceServer::new(self);
        return node_service;
    }
}

#[tonic::async_trait]
impl NodeWriteService for NodeWrite {
    async fn create_transaction(
        &self,
        request: Request<CreateTransactionRequest>,
    ) -> Result<Response<TransactionRecord>, Status> {
        let transaction_request = request.into_inner();

        if PublicKey::from_str(&transaction_request.sender_public_key).is_err() {
            return Err(Status::internal(format!("Cannot parse sender_public_key")));
        }
        if Signature::from_str(&transaction_request.signature).is_err() {
            return Err(Status::internal(format!("Cannot parse signature")));
        }

        let new_txn_args = NewTxnArgs::from(transaction_request);
        let txn = Txn::new(new_txn_args);
        let event = Event::NewTxnCreated(txn.clone());

        self.events_tx
            .send(event.into())
            .await
            .map_err(|e| Status::internal(format!("Internal error: {}", e)))?;

        Ok(Response::new(TransactionRecord::from(
            RpcTransactionRecord::from(txn),
        )))
    }
}
