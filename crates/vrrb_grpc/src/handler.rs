include!("gen/mod.rs");

use std::collections::HashMap;

use helloworld::v1::{
    hello_world_service_server::{HelloWorldService, HelloWorldServiceServer},
    SayHelloRequest,
    SayHelloResponse,
};
use mempool::MempoolReadHandleFactory;
use node::v1::{
    node_service_server::{NodeService, NodeServiceServer},
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
use tonic::{transport::Server, Request, Response, Status};
use vrrb_core::{
    account::Account,
    txn::{NewTxnArgs, Token, TxAmount, TxNonce, TxTimestamp, Txn},
};

use crate::server::GRPCServerConfig;

#[derive(Debug, Default)]
pub struct MyHelloWorld {}

impl MyHelloWorld {
    pub fn init() -> HelloWorldServiceServer<MyHelloWorld> {
        let helloworld_handler = MyHelloWorld::default();
        let helloworld_service = HelloWorldServiceServer::new(helloworld_handler);
        return helloworld_service;
    }
}

#[tonic::async_trait]
impl HelloWorldService for MyHelloWorld {
    async fn say_hello(
        &self,
        request: Request<SayHelloRequest>,
    ) -> Result<Response<SayHelloResponse>, Status> {
        let response = SayHelloResponse {
            message: format!("Hello, {}!", request.get_ref().name),
        };
        Ok(Response::new(response))
    }
}

#[derive(Debug)]
pub struct Node {
    pub node_type: NodeType,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
    // pub events_tx: EventPublisher,
}

impl Node {
    pub fn init(self) -> NodeServiceServer<Node> {
        let node_service = NodeServiceServer::new(self);
        return node_service;
    }
}

#[tonic::async_trait]
impl NodeService for Node {
    async fn get_node_type(
        &self,
        request: Request<GetNodeTypeRequest>,
    ) -> Result<Response<GetNodeTypeResponse>, Status> {
        let response = GetNodeTypeResponse {
            id: (self.node_type as i32).to_string(),
            result: self.node_type.to_string(),
        };
        Ok(Response::new(response))
    }

    async fn get_full_mempool(
        &self,
        request: Request<GetFullMempoolRequest>,
    ) -> Result<Response<GetFullMempoolResponse>, Status> {
        let values: Vec<RpcTransactionRecord> = self
            .mempool_read_handle_factory
            .values()
            .iter()
            .map(|txn| RpcTransactionRecord::from(txn.clone()))
            .collect();

        let transaction_records = values
            .iter()
            .map(|transaction_record| TransactionRecord::from(transaction_record.clone()))
            .collect();

        let response = GetFullMempoolResponse {
            transaction_records,
        };

        return Ok(Response::new(response));
    }
}

//////////////////////////////////////////////////////

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
