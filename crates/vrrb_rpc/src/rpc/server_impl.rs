use std::net::SocketAddr;

use async_trait::async_trait;
use jsonrpsee::{
    core::Error,
    server::{ServerBuilder, SubscriptionSink},
    types::SubscriptionResult,
};
use mempool::MempoolReadHandleFactory;
use primitives::NodeType;
use storage::vrrbdb::VrrbDbReadHandle;
use tokio::sync::mpsc::UnboundedSender;
use vrrb_core::{
    account::Account,
    event_router::{DirectedEvent, Event, Topic},
    txn::NewTxnArgs,
};

use super::api::{CreateTxnArgs, FullMempoolSnapshot};
use crate::rpc::api::{FullStateSnapshot, RpcServer};

pub struct RpcServerImpl {
    pub node_type: NodeType,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
    pub events_tx: UnboundedSender<DirectedEvent>,
}

#[async_trait]
impl RpcServer for RpcServerImpl {
    async fn get_full_state(&self) -> Result<FullStateSnapshot, Error> {
        let values = self.vrrbdb_read_handle.state_store_values();

        Ok(values)
    }

    async fn get_full_mempool(&self) -> Result<FullMempoolSnapshot, Error> {
        let values = self.mempool_read_handle_factory.values();

        Ok(values)
    }

    async fn get_node_type(&self) -> Result<NodeType, Error> {
        Ok(self.node_type)
    }

    async fn create_txn(&self, args: CreateTxnArgs) -> Result<(), Error> {
        let txn = vrrb_core::txn::Txn::new(NewTxnArgs {
            sender_address: args.sender_address,
            sender_public_key: args.sender_public_key,
            receiver_address: args.receiver_address,
            token: args.token,
            amount: args.amount,
            payload: args.payload,
            signature: args.signature,
            validators: None,
            nonce: args.nonce,
        });

        let event = Event::NewTxnCreated(txn);

        self.events_tx
            .send((Topic::Transactions, event))
            .map_err(|err| {
                telemetry::error!("could not queue transaction to mempool: {err}");
                Error::Custom(err.to_string())
            })?;

        Ok(())
    }

    async fn create_account(&self, account: Account) -> Result<(), Error> {
        todo!()
    }

    async fn update_account(&self, account: Account) -> Result<(), Error> {
        todo!()
    }

    async fn get_transaction(&self, transaction_digest: TransactionDigest) -> Result<(), Error> {}

    async fn list_transactions(&self) -> Result<(), Error> {}

    async fn create_account(&self, account: Account) -> Result<(), Error> {}

    async fn update_account(&self, account: Account) -> Result<(), Error> {}
}
